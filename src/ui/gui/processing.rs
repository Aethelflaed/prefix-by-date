use crate::matcher::Matcher;
use crate::processing::{
    self, Communication, Confirmation, Error, Processing, Reporter,
};
use crate::replacement::Replacement;

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use iced::futures;

use futures::channel::mpsc;
use futures::executor::block_on;
use futures::lock::Mutex;
use futures::sink::SinkExt;
use futures::stream::FusedStream;
use futures::Stream;
use futures::StreamExt;

#[derive(Debug, Clone)]
pub enum Event {
    Initialization(Connection<InitializationData>),
    Ready(Connection),
    Processing(PathBuf),
    ProcessingOk(Replacement),
    ProcessingErr(PathBuf, String),
    Confirm(Replacement),
    Rescue(Replacement),
    Finished,
    Aborted,
}

#[derive(Debug, Clone)]
pub enum InitializationData {
    Matchers(Vec<Box<dyn Matcher>>),
    Paths(Vec<PathBuf>),
    Done,
}

pub fn connect() -> impl Stream<Item = Event> {
    iced::stream::channel(100, |mut output| async move {
        let (gui_tx, mut gui_rx) = mpsc::channel::<InitializationData>(100);
        output
            .send(Event::Initialization(Connection(gui_tx)))
            .await
            .expect("Send connection to UI");

        let mut matchers = Vec::<Box<dyn Matcher>>::new();
        let mut paths = Vec::<PathBuf>::new();

        loop {
            match gui_rx.next().await {
                Some(InitializationData::Matchers(m)) => matchers = m,
                Some(InitializationData::Paths(p)) => paths = p,
                Some(InitializationData::Done) => break,
                None => panic!("Connection to UI broke during initialization"),
            }
        }

        // Create channel to communicate the confirmation back
        // to the GUI
        let (gui_tx, mut gui_rx) = mpsc::channel::<Confirmation>(100);

        // Send the gui_tx back to the application
        output
            .send(Event::Ready(Connection(gui_tx)))
            .await
            .expect("Send connection to UI");

        let (mut worker_tx, mut worker_rx) = mpsc::channel::<Event>(100);

        // We are ready to receive confirmation messages.
        // Now we can create the processing on another thread
        std::thread::spawn(move || {
            let front = ProcessingFront::new(&mut gui_rx, worker_tx.clone());
            let result = match Processing::new(&front, &matchers, &paths).run()
            {
                Ok(_) => Event::Finished,
                Err(_) => Event::Aborted,
            };

            if !worker_tx.is_closed() {
                block_on(worker_tx.send(result))
                    .expect("Send message on channel");
            }
        });

        // Now we loop for events to send to the GUI
        loop {
            // The processing thread might finish, which would drop all
            // the worker_tx, so we need to check if it's terminated here
            if worker_rx.is_terminated() {
                break;
            }

            if let Some(event) = worker_rx.next().await {
                output.send(event).await.expect("Send message to UI");
            }
        }

        loop {
            // channel need an infallible future, so we
            // just loop indefinitely.
            // We sleep a whole day to yield control to the executor
            tokio::time::sleep(tokio::time::Duration::from_secs(86_400)).await;
        }
    })
}

#[derive(Debug, Clone)]
pub struct Connection<T = Confirmation>(mpsc::Sender<T>);

impl<T> Connection<T> {
    pub async fn send_async(&mut self, payload: T) {
        self.0
            .send(payload)
            .await
            .expect("Send confirmation to processing");
    }
}

struct ProcessingFront<'a> {
    gui_rx: Mutex<&'a mut mpsc::Receiver<Confirmation>>,
    worker_tx: RefCell<mpsc::Sender<Event>>,
}

impl<'a> ProcessingFront<'a> {
    pub fn new(
        gui_rx: &'a mut mpsc::Receiver<Confirmation>,
        worker_tx: mpsc::Sender<Event>,
    ) -> ProcessingFront<'a> {
        Self {
            gui_rx: Mutex::new(gui_rx),
            worker_tx: RefCell::new(worker_tx),
        }
    }

    // Only return false if the channel is closed
    fn send(&self, event: Event) -> bool {
        let mut worker_tx = self.worker_tx.borrow_mut();
        if !worker_tx.is_closed() {
            block_on(worker_tx.send(event))
                .expect("Send event from processing thread");

            true
        } else {
            false
        }
    }
}

impl<'a> Reporter for ProcessingFront<'a> {
    fn setup(&self, _count: usize) {}
    fn processing(&self, path: &Path) {
        self.send(Event::Processing(path.to_path_buf()));
    }
    fn processing_ok(&self, replacement: &Replacement) {
        self.send(Event::ProcessingOk(replacement.clone()));
    }
    fn processing_err(&self, path: &Path, error: &Error) {
        self.send(Event::ProcessingErr(
            path.to_path_buf(),
            format!("{}", error),
        ));
    }
}

impl<'a> Communication for ProcessingFront<'a> {
    fn confirm(&self, replacement: &Replacement) -> Confirmation {
        if !self.send(Event::Confirm(replacement.clone())) {
            return Confirmation::Abort;
        }

        let receiving = async { self.gui_rx.lock().await.next().await };
        // If we don't get a confirmation, it means the UI is quitting, so we
        // abort
        block_on(receiving).unwrap_or(Confirmation::Abort)
    }

    fn rescue(&self, error: Error) -> processing::Result<Replacement> {
        match &error {
            Error::NoMatch(path) => {
                let replacement = match Replacement::try_from(path.as_path()) {
                    Ok(rep) => rep,
                    Err(_) => return Err(error),
                };

                if !self.send(Event::Rescue(replacement.clone())) {
                    return Err(Error::Abort);
                }

                let receiving = async { self.gui_rx.lock().await.next().await };
                // If we don't get a confirmation, it means the UI is
                // quitting, so we abort
                let conf = match block_on(receiving) {
                    None => return Err(Error::Abort),
                    Some(conf) => conf,
                };
                match conf {
                    // If we receive Confirmation::Abort, this means the rescue
                    // is aborted, so we return the original error
                    Confirmation::Abort => Err(Error::Abort),
                    Confirmation::Replace(replacement) => Ok(replacement),
                    Confirmation::Skip | Confirmation::Refuse => Err(error),
                    other => {
                        log::warn!(
                            "Unexpected rescue confirmation: {:?}",
                            other
                        );
                        Err(error)
                    }
                }
            }
            _ => {
                log::warn!("Unexpected rescue: {:?}", error);
                Err(error)
            }
        }
    }
}
