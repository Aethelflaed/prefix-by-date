use crate::matcher::Matcher;
use crate::processing::{Communication, Confirmation, Error, Processing};
use crate::replacement::Replacement;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use iced::futures;
use iced::subscription::{self, Subscription};

use futures::channel::mpsc as iced_mpsc;
use futures::executor::block_on;
use futures::sink::SinkExt;

#[derive(Debug, Clone)]
pub enum Event {
    Ready(mpsc::Sender<Confirmation>),
    Processing(PathBuf),
    ProcessingOk(Replacement),
    ProcessingErr(PathBuf, String),
    Confirm(Replacement),
}

#[derive(Debug)]
enum State {
    Starting,
    Ready(mpsc::Receiver<Confirmation>),
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
            let mut state = State::Starting;

            loop {
                log::error!("State: {:?}", state);
                match &mut state {
                    State::Starting => {
                        // Create channel
                        let (sender, receiver) = mpsc::channel();

                        // Send the sender back to the application
                        output.send(Event::Ready(sender)).await.unwrap();

                        // We are ready to receive messages
                        state = State::Ready(receiver);
                    }
                    State::Ready(receiver) => {
                        let front = Front::new(&mut output, receiver);
                        Processing::new(&front, &matchers, &paths)
                            .run()
                            .unwrap()
                    }
                }
            }
        },
    )
}

struct Front<'a> {
    output: RefCell<&'a mut iced_mpsc::Sender<Event>>,
    receiver: &'a mpsc::Receiver<Confirmation>,
}

impl<'a> Front<'a> {
    pub fn new(
        output: &'a mut iced_mpsc::Sender<Event>,
        receiver: &'a mpsc::Receiver<Confirmation>,
    ) -> Front<'a> {
        Self {
            output: RefCell::new(output),
            receiver,
        }
    }
}

impl<'a> Communication for Front<'a> {
    fn processing(&self, path: &Path) {
        let mut binding = self.output.borrow_mut();
        let sent = binding.send(Event::Processing(path.to_path_buf()));

        block_on(sent).unwrap();
    }
    fn processing_ok(&self, replacement: &Replacement) {
        let mut binding = self.output.borrow_mut();
        let sent = binding.send(Event::ProcessingOk(replacement.clone()));

        block_on(sent).unwrap();
    }
    fn processing_err(&self, path: &Path, error: &Error) {
        let mut binding = self.output.borrow_mut();
        let sent = binding.send(Event::ProcessingErr(
            path.to_path_buf(),
            format!("{}", error),
        ));

        block_on(sent).unwrap();
    }
    fn confirm(&self, replacement: &Replacement) -> Confirmation {
        let mut binding = self.output.borrow_mut();
        let sent = binding.send(Event::Confirm(replacement.clone()));

        block_on(sent).unwrap();
        self.receiver.recv().unwrap()
    }
}
