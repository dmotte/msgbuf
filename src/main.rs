use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvError, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

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

const CHAN_BUF_SIZE: usize = 4096;
const MIN_RECV_TIME: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
enum InputMode {
    /// Process all bytes
    Binary,
    /// Process ASCII text only (bytes 0x00..0x7f) and ignore the rest
    Ascii,
}

/// Message buffer
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable debug messages
    #[arg(short, long)]
    debug: bool,

    /// Input mode
    #[arg(short = 'I', long, value_enum, default_value = "binary")]
    input_mode: InputMode,

    /// Minimum interval (in seconds) between checks for messages to be sent
    #[arg(short, long, default_value_t = 60)]
    interval: u64,

    /// Maximum message length in bytes
    #[arg(short, long, default_value_t = 1024)]
    max_msg_len: usize,

    /// Notifier command that will be invoked as a child process whenever a
    /// message needs to be sent. The text of the message will be written to
    /// the stdin of the child process
    #[arg(last = true)]
    notifier: Vec<String>,
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<()> {
    let args = Args::parse();
    if args.debug {
        println!("DEBUG: {args:?}");
    }

    let interval = Duration::from_secs(args.interval);

    if args.max_msg_len == 0 {
        bail!("the maximum message length must be > 0");
    }

    if args.notifier.is_empty() {
        bail!("the notifier command cannot be empty");
    }

    // Note: we need the following additional channel and thread to offload
    // reading from stdin because accessing io::stdin() directly is inherently
    // blocking by default, and we need a non-blocking way in the main thread

    let (tx, rx) = mpsc::sync_channel(CHAN_BUF_SIZE);

    let hnd_read_and_send = thread::spawn(move || -> Result<()> {
        let stdin = io::stdin();
        let handle = stdin.lock();
        for byte in handle.bytes() {
            let b = byte.context("failed to read byte from stdin")?;
            if args.input_mode == InputMode::Ascii && b > 0x7f {
                continue;
            }
            tx.send(b)
                .context("failed to send byte to internal channel")?;
        }

        Ok(())
    });

    let mut msg = vec![0u8; args.max_msg_len];
    let mut msg_len: usize = 0;
    let mut stdin_eof = false;
    let mut next_invocation = Instant::now();

    if args.debug {
        println!("DEBUG: recv_and_invoke loop starting");
    }

    'recv_and_invoke: loop {
        // STEP: if msg is empty, start RECEIVING one byte (blocking)

        if !stdin_eof && msg_len == 0 {
            if args.debug {
                println!("DEBUG: msg is empty; receiving first byte");
            }

            // Note: the call to recv is blocking
            match rx.recv() {
                Ok(b) => {
                    msg[0] = b;
                    msg_len = 1;
                }
                Err(RecvError) => stdin_eof = true,
            }

            // Give the sender some time to send everything it has to
            thread::sleep(MIN_RECV_TIME);
        }

        // STEP: WAIT some time before proceeding, if needed

        let now = Instant::now();
        if next_invocation < now {
            // We're "behind schedule", so let's reset next_invocation
            next_invocation = now + interval;
        } else {
            thread::sleep(next_invocation - now);
            next_invocation += interval;
        }

        // STEP: RECEIVE the rest, until msg is full or EOF is reached

        'nonblocking_recv: while !stdin_eof && msg_len < args.max_msg_len {
            // Note: the call to try_recv is non-blocking
            match rx.try_recv() {
                Ok(b) => {
                    msg[msg_len] = b;
                    msg_len += 1;
                }
                Err(TryRecvError::Empty) => break 'nonblocking_recv,
                Err(TryRecvError::Disconnected) => stdin_eof = true,
            }
        }

        if stdin_eof && msg_len == 0 {
            break 'recv_and_invoke;
        }

        // STEP: INVOKE the notifier command

        if args.debug {
            println!(
                "DEBUG: invoking notifier command with input: \"{}\"",
                escape_ascii(&msg[..msg_len]),
            );
        }

        let mut child = Command::new(&args.notifier[0])
            .args(&args.notifier[1..])
            .stdin(Stdio::piped())
            .spawn()
            .context("failed to spawn notifier command")?;
        child
            .stdin
            .take()
            .context("failed to acquire notifier process stdin")?
            .write_all(&msg[..msg_len])
            .context("failed to write message to notifier process")?;
        let exit_status = child
            .wait()
            .context("failed to wait for notifier process to exit")?;
        if exit_status.success() {
            msg_len = 0;
        } else {
            match exit_status.code() {
                Some(code) => {
                    eprintln!("The notifier command returned non-zero exit status {code}");
                }
                None => eprintln!("The notifier command was terminated by a signal"),
            }
        }

        if stdin_eof && msg_len == 0 {
            break 'recv_and_invoke;
        }
    }

    if args.debug {
        println!("DEBUG: recv_and_invoke loop finished");
    }

    hnd_read_and_send
        .join()
        .map_err(|panic| anyhow::anyhow!("the stdin reader thread panicked: {panic:?}"))?
        .context("the stdin reader thread returned an error")?;

    Ok(())
}
