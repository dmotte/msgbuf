use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use log::{Level, debug, error, log_enabled};

use crate::notifier;

// Src: https://github.com/dmotte/misc/tree/main/snippets
fn escape_ascii(bytes: &[u8]) -> String {
    // Pre-allocate space for at least the length of the input bytes
    let mut result = String::with_capacity(bytes.len());

    result.extend(
        bytes
            .iter()
            .flat_map(|&b| std::ascii::escape_default(b))
            .map(|b| b as char),
    );

    result
}

/// Minimum receive time
const MIN_RECV_TIME: Duration = Duration::from_millis(100);

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to invoke notifier: {0}")]
    NotifierInvoke(#[source] notifier::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct App {
    /// Buffer used to store the bytes that need to be sent
    buf: Vec<u8>,
    /// Length of the whole stored bytes
    len_all: usize,
    /// Length of the bytes that can be sent, based on [`Self::split_on_byte`]
    len_send: usize,

    /// Receiving half of the channel, used to receive the incoming bytes
    rx: Receiver<u8>,

    /// If set, message splits will preferably occur on this byte value
    split_on_byte: Option<u8>,

    /// Minimum interval between send attempts
    interval_send: Duration,
    /// Time instant when the next send will be possible
    next_send: Instant,
    /// Path of the notifier command
    notifier_program: String,
    /// Arguments to be passed to the notifier command
    notifier_args: Vec<String>,
}

impl App {
    pub fn new(
        size: usize,
        rx: Receiver<u8>,
        split_on_byte: Option<u8>,
        interval_send: Duration,
        notifier_program: impl Into<String>,
        notifier_args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            buf: vec![0u8; size],
            len_all: 0,
            len_send: 0,

            rx,

            split_on_byte,

            interval_send,
            next_send: Instant::now(),
            notifier_program: notifier_program.into(),
            notifier_args: notifier_args.into_iter().map(Into::into).collect(),
        }
    }

    /// Adds a byte to the buffer. Warning: this does NOT check that there is
    /// space in the buffer before adding. If the buffer is full, this
    /// function will panic.
    fn add_byte(&mut self, byte: u8) {
        self.buf[self.len_all] = byte;
        self.len_all += 1;

        if self.split_on_byte.is_none() || self.split_on_byte == Some(byte) {
            self.len_send = self.len_all;
        }
    }

    /// Tries to send some bytes by invoking the notifier. Returns
    /// [`true`](primitive@bool) if the bytes were successfully sent,
    /// [`false`](primitive@bool) if the notifier failed with some
    /// [`std::process::ExitStatus`], or an [`Error`] if one occurred.
    fn try_send(&mut self, len: usize) -> Result<bool> {
        assert_ne!(len, 0);
        assert!(len <= self.len_all);

        self.next_send = Instant::now() + self.interval_send;

        if log_enabled!(Level::Debug) {
            let escaped = escape_ascii(&self.buf[..len]);
            debug!("invoking notifier command with input: \"{escaped}\"");
        }

        let status = notifier::invoke(
            &self.notifier_program,
            &self.notifier_args,
            &self.buf[..len],
        )
        .map_err(Error::NotifierInvoke)?;

        if status.success() {
            self.buf.copy_within(len..self.len_all, 0);
            self.len_all -= len;
            self.len_send = self.len_send.saturating_sub(len);

            Ok(true)
        } else {
            if let Some(code) = status.code() {
                error!("notifier command failed with status code {code}");
            } else {
                error!("notifier command terminated by a signal");
            }

            Ok(false)
        }
    }

    /// Sleeps a bit to give the sender some time to send everything it has to
    fn sleep_for_recv() {
        thread::sleep(MIN_RECV_TIME);
    }

    /// Sleeps for the time required before the next send, if needed
    fn sleep_for_send(&self) {
        let now = Instant::now();

        if now < self.next_send {
            thread::sleep(self.next_send - now);
        }
    }

    /// Runs the main app logic.
    ///
    /// This method intentionally takes ownership of [`self`] (`mut self`)
    /// instead of passing by mutable reference (`&mut self`), to prevent
    /// calling this function on an already-run instance. The goal is to
    /// enforce a "one-shot" execution lifecycle and ensure that, once this
    /// function finishes or encounters an error, the instance is
    /// cleanly dropped and cannot be reused or re-run later.
    pub fn run(mut self) -> Result<()> {
        'recv_blocking: loop {
            // If we are here, it means that:
            //   - self.buf is NOT full, AND
            //   - self.len_send == 0, i.e. we cannot send anything
            // Based on that, we block on receiving a byte.

            debug!("blocking on receiving byte");

            let Ok(byte) = self.rx.recv() else {
                break 'recv_blocking; // EOF has been reached
            };

            self.add_byte(byte);

            // Is self.buf full?
            if self.len_all == self.buf.len() {
                // If we are here, it means that the self.buf is full. Plus, we
                // know that, before receiving the last byte, self.len_send was
                // zero (i.e. we could not send anything). That means that now
                // self.len_send can either be still zero OR == self.len_all.
                // In both cases, we now have to send the whole buffer content
                // (self.len_all), as we cannot receive any bytes anymore.
                loop {
                    self.sleep_for_send();

                    if self.try_send(self.len_all)? {
                        // Send successful. We know that we cannot send
                        // anything more now, so let's go back at
                        // receiving in blocking mode
                        continue 'recv_blocking;
                    }
                }
            }

            // If we are here, it means that self.buf is still NOT full

            if self.len_send == 0 {
                continue 'recv_blocking; // We still cannot send anything
            }

            // If we are here, it means that self.len_send > 0 (i.e. we could
            // send something)

            'sleep_recv_polling: loop {
                Self::sleep_for_recv();

                self.sleep_for_send();

                'recv_polling: loop {
                    // Try to receive as many bytes as possible, until one of
                    // the following happens:
                    //   - self.buf becomes full
                    //   - we don't see any bytes incoming anymore
                    //     (i.e. TryRecvError::Empty)
                    //   - we reach EOF

                    match self.rx.try_recv() {
                        Ok(byte) => {
                            self.add_byte(byte);

                            // Is self.buf full?
                            if self.len_all == self.buf.len() {
                                // The buffer self.buf is full. We cannot
                                // receive anything anymore now, only send
                                break 'sleep_recv_polling;
                            }

                            // If we are here, it means that self.buf is
                            // NOT full yet. Let's see if we can receive some
                            // more bytes immediately, without sleeping
                            #[allow(clippy::needless_continue)]
                            continue 'recv_polling;
                        }
                        Err(TryRecvError::Empty) => {
                            // We have some bytes to send and we don't see any
                            // more bytes incoming. Let's send what we have now

                            if !self.try_send(self.len_send)? {
                                // Send failed. We know that we still have
                                // something to send now, but self.buf is NOT
                                // full yet, so let's go sleeping as needed
                                // now, then we will try to receive some more
                                // bytes (just in case), and eventually retry
                                // to send again
                                continue 'sleep_recv_polling;
                            }

                            // Send successful. We know that we cannot send
                            // anything more now, so let's go back at
                            // receiving in blocking mode
                            continue 'recv_blocking;
                        }
                        Err(TryRecvError::Disconnected) => {
                            break 'recv_blocking; // EOF has been reached
                        }
                    }
                }
            }

            // If we are here, it means that self.len_send > 0 (i.e. we can
            // send something), but self.buf is full, so we cannot
            // receive anything anymore now, only send

            loop {
                self.sleep_for_send();

                if self.try_send(self.len_send)? {
                    // Send successful. We know that we cannot send
                    // anything more now, so let's go back at
                    // receiving in blocking mode
                    continue 'recv_blocking;
                }
            }
        }

        // If we are here, it means that EOF has been reached

        if self.len_all == 0 {
            return Ok(());
        }

        // There is still something in the buffer. Send ALL the remaining
        // bytes, regardless of self.len_send
        loop {
            self.sleep_for_send();

            if self.try_send(self.len_all)? {
                break;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::mpsc;

    #[test]
    fn empty_input_succeeds_without_invoking_notifier() -> Result<()> {
        let (tx, rx) = mpsc::channel();

        drop(tx); // Close the sender half of the channel

        // Running with empty input should finish without even trying to
        // invoke the notifier
        App::new(
            10,
            rx,
            None,
            Duration::ZERO,
            "/nonexistent/program",
            std::iter::empty::<String>(),
        )
        .run()?;

        Ok(())
    }
}
