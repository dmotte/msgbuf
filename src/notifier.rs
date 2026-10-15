use std::ffi::OsStr;
use std::io::{self, Write};
use std::process::{Command, ExitStatus, Stdio};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to spawn notifier command: {0}")]
    Spawn(#[source] io::Error),

    #[error("failed to acquire notifier process stdin")]
    StdinMissing,

    #[error("failed to write message to notifier process: {0}")]
    StdinWrite(#[source] io::Error),

    #[error("failed to wait for notifier process to exit: {0}")]
    Wait(#[source] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn invoke<S, I>(program: S, args: I, msg: &[u8]) -> Result<ExitStatus>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(Error::Spawn)?;

    let mut stdin = child.stdin.take().ok_or(Error::StdinMissing)?;
    stdin.write_all(msg).map_err(Error::StdinWrite)?;
    drop(stdin); // Close the stdin stream handle

    child.wait().map_err(Error::Wait)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn writes_something() -> Result<()> {
        let status = invoke("cat", [], b"Hello, World!")?;

        assert!(status.success());

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn writes_nothing() -> Result<()> {
        let status = invoke("cat", [], b"")?;

        assert!(status.success());

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn returns_status_code() -> Result<()> {
        let status = invoke("sh", ["-c", "exit 7"], b"")?;

        assert!(!status.success());
        assert_eq!(status.code(), Some(7));

        Ok(())
    }

    #[test]
    fn returns_error_for_nonexistent_program() -> Result<()> {
        let result = invoke("/nonexistent/program", [], b"");

        assert!(matches!(result, Err(Error::Spawn(_))));

        Ok(())
    }
}
