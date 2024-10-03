use anyhow::Result;
use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::str;

pub struct Env {
    pub conf_dir: TempDir,
}

impl Env {
    pub fn new() -> Result<Self> {
        Ok(Self {
            conf_dir: TempDir::new()?.into_persistent_if(
                std::env::var_os("TEST_PERSIST_FILES").is_some(),
            ),
        })
    }

    pub fn command(&self) -> Result<Command> {
        let mut cmd = Command::cargo_bin("prefix-by-date")?;
        cmd.arg("-C").arg(self.conf_dir.path());
        Ok(cmd)
    }
}

#[test]
fn empty() -> Result<()> {
    let env = Env::new()?;

    env.command()?
        .assert()
        .success()
        .stderr(str::is_empty());

    Ok(())
}
