#![cfg(feature = "notif")]

use crate::processing::{Error, Reporter};
use crate::replacement::Replacement;

use std::cell::{Cell, RefCell};
use std::path::Path;

use notify_rust::{
    get_capabilities, Hint, Notification, NotificationHandle, Timeout,
};

pub struct NotifReporter {
    count: Cell<usize>,
    current: Cell<usize>,
    notification: RefCell<Option<NotificationHandle>>,
}

impl NotifReporter {
    fn inc_progress(&self) {
        self.current.set(self.current.get() + 1);
    }

    fn progress_bar(&self) -> String {
        format!("Processing {}/{}", self.current.get(), self.count.get(),)
    }

    fn update_message(&self, path: Option<&Path>) {
        if let Some(mut notif) = self.notification.take() {
            if let Some(path) = &path {
                notif.body(
                    format!(
                        "Processing {:?}.<br>{}",
                        path,
                        self.progress_bar()
                    )
                    .as_str(),
                );
            } else {
                notif.body(self.progress_bar().as_str());
            }

            notif.update();

            self.notification.replace(Some(notif));
        }
    }
}

impl Drop for NotifReporter {
    fn drop(&mut self) {
        if let Some(notif) = self.notification.take() {
            notif.close();
        }
    }
}

impl Default for NotifReporter {
    fn default() -> Self {
        if let Ok(caps) = get_capabilities() {
            log::debug!("Notification capabilities: {:?}", caps);
        }

        Self {
            count: Default::default(),
            current: Default::default(),
            notification: Default::default(),
        }
    }
}

impl Reporter for NotifReporter {
    /// Report the total count of elements about to be processed
    fn setup(&self, count: usize) {
        self.count.set(count);
        let notif = Notification::new()
            .summary("Prefix by date")
            .body("Processing files")
            .hint(Hint::Category("transfer".to_owned()))
            .hint(Hint::Resident(true))
            .timeout(Timeout::Never)
            .show()
            .ok();

        self.notification.replace(notif);
    }

    /// Start processing this path
    fn processing(&self, path: &Path) {
        self.update_message(Some(path));
    }

    /// Processing went well and ended-up with this replacement
    fn processing_ok(&self, _replacement: &Replacement) {
        self.inc_progress();
    }

    /// Processing encountered this error
    fn processing_err(&self, _path: &Path, _error: &Error) {
        self.inc_progress();
    }
}
