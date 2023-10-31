use env_logger::{Builder, Env};
use systemd_journal_logger::{connected_to_journal, JournalLog};

pub fn setup() -> std::io::Result<()> {
    if connected_to_journal() {
        // If the output streams of this process are directly connected to the
        // systemd journal log directly to the journal to preserve structured
        // log entries (e.g. proper multiline messages, metadata fields, etc.)
        JournalLog::new()
            .unwrap()
            .with_extra_fields(vec![("VERSION", env!("CARGO_PKG_VERSION"))])
            .install()
            .unwrap();
    } else {
        let env = Env::new()
            .filter(format!("{}_LOG", env!("CARGO_PKG_NAME")))
            .write_style(format!("{}_LOG_STYLE", env!("CARGO_PKG_NAME")));

        Builder::new()
            .filter_level(log::LevelFilter::Trace)
            .parse_env(env)
            .init();
    }

    Ok(())
}
