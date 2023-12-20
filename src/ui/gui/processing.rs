use crate::matcher::Matcher;
use crate::processing::{Communication, Confirmation, Error, Processing};
use crate::replacement::Replacement;

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use iced::futures;
use iced::subscription::{self, Subscription};

use futures::channel::mpsc;
use futures::executor::block_on;
use futures::lock::Mutex;
use futures::sink::SinkExt;
use futures::StreamExt;

#[derive(Debug, Clone)]
pub enum Event {
    Ready(Connection),
    Processing(PathBuf),
    ProcessingOk(Replacement),
    ProcessingErr(PathBuf, String),
    Confirm(Replacement),
    Finished,
    Aborted,
}

pub fn connect(
    matchers: &[Box<dyn Matcher>],
    paths: &[PathBuf],
) -> Subscription<Event> {
    struct Connect;

    let matchers = matchers.to_owned();
    let paths = paths.to_owned();

    subscription::channel(
        std::any::TypeId::of::<Connect>(),
        100,
        |mut output| async move {
            // Create channel to communicate the confirmation back
            // to the GUI
            let (conf_tx, mut conf_rx) = mpsc::channel(100);

            // Send the conf_tx back to the application
            output
                .send(Event::Ready(Connection(conf_tx)))
                .await
                .unwrap();

            let (mut event_tx, mut event_rx) = mpsc::channel(100);

            // We are ready to receive confirmation messages.
            // Now we can create the processing on another thread
            std::thread::spawn(move || {
                let front =
                    ProcessingFront::new(&mut conf_rx, event_tx.clone());
                let sending =
                    match Processing::new(&front, &matchers, &paths).run() {
                        Ok(_) => event_tx.send(Event::Finished),
                        Err(_) => event_tx.send(Event::Aborted),
                    };

                block_on(sending).expect("Send message on channel");
            });

            loop {
                if let Some(event) = event_rx.next().await {
                    output.send(event).await.unwrap();
                }
            }
        },
    )
}

#[derive(Debug, Clone)]
pub struct Connection(mpsc::Sender<Confirmation>);

impl Connection {
    pub fn send(&mut self, confirmation: Confirmation) {
        self.0
            .try_send(confirmation)
            .expect("Send confirmation to processing")
    }
}

struct ProcessingFront<'a> {
    conf_rx: Mutex<&'a mut mpsc::Receiver<Confirmation>>,
    event_tx: RefCell<mpsc::Sender<Event>>,
}

impl<'a> ProcessingFront<'a> {
    pub fn new(
        conf_rx: &'a mut mpsc::Receiver<Confirmation>,
        event_tx: mpsc::Sender<Event>,
    ) -> ProcessingFront<'a> {
        Self {
            conf_rx: Mutex::new(conf_rx),
            event_tx: RefCell::new(event_tx),
        }
    }
}

impl<'a> Communication for ProcessingFront<'a> {
    fn processing(&self, path: &Path) {
        log::info!("Processing: {:?}", path);

        let mut event_tx = self.event_tx.borrow_mut();
        let sending = event_tx.send(Event::Processing(path.to_path_buf()));

        block_on(sending).expect("Send message on channel");
    }
    fn processing_ok(&self, replacement: &Replacement) {
        log::info!("Processing ok: {:}", replacement);

        let mut event_tx = self.event_tx.borrow_mut();
        let sending = event_tx.send(Event::ProcessingOk(replacement.clone()));

        block_on(sending).expect("Send message on channel");
    }
    fn processing_err(&self, path: &Path, error: &Error) {
        log::info!("Processing error: {:?}: {:?}", path, error);

        let mut event_tx = self.event_tx.borrow_mut();
        let sending = event_tx.send(Event::ProcessingErr(
            path.to_path_buf(),
            format!("{}", error),
        ));

        block_on(sending).expect("Send message on channel");
    }
    fn confirm(&self, replacement: &Replacement) -> Confirmation {
        let mut event_tx = self.event_tx.borrow_mut();
        let sending = event_tx.send(Event::Confirm(replacement.clone()));

        block_on(sending).expect("Send message on channel");
        log::info!("Confirming replacement: {:}", replacement);

        let receiving =
            async { self.conf_rx.lock().await.select_next_some().await };
        block_on(receiving)
    }
}
