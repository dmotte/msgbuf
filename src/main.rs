mod app;
mod notifier;

use std::io::{self, Read};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use log::debug;

use app::App;

/// Bounded channel internal buffer size multiplier
const CHAN_SIZE_MULT: usize = 2;

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
    /// Input mode
    #[arg(short = 'I', long, value_enum, default_value = "binary")]
    input_mode: InputMode,

    /// Minimum receive interval (in milliseconds), to give the sender some
    /// time to send everything it has to
    #[arg(short = 'r', long, default_value_t = 100)]
    interval_recv: u64,

    /// Minimum interval (in milliseconds) between notifier command invocations
    #[arg(short, long, default_value_t = 60_000)]
    interval_send: u64,

    /// If set, message splits will preferably occur on this byte value
    #[arg(short, long)]
    split_on_byte: Option<u8>,

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
    env_logger::builder().format_timestamp(None).init();

    let args = Args::parse();

    debug!("parsed command line arguments: {args:?}");

    if args.max_msg_len == 0 {
        bail!("the maximum message length must be > 0");
    }

    if args.notifier.is_empty() {
        bail!("the notifier command cannot be empty");
    }

    // Note: we need the following additional channel and thread to offload
    // reading from stdin because accessing io::stdin() directly is inherently
    // blocking by default, and we need a non-blocking way in the main thread

    let (tx, rx) = mpsc::sync_channel(args.max_msg_len * CHAN_SIZE_MULT);

    let stdin_reader = thread::Builder::new()
        .name("stdin-reader".to_string())
        .spawn(move || -> Result<()> {
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
        })
        .context("failed to spawn stdin-reader thread")?;

    debug!("app run starting");

    App::new(
        args.max_msg_len,
        rx,
        args.split_on_byte,
        Duration::from_millis(args.interval_recv),
        Duration::from_millis(args.interval_send),
        &args.notifier[0],
        &args.notifier[1..],
    )
    .run()?;

    debug!("app run finished");

    stdin_reader
        .join()
        .map_err(|panic| anyhow::anyhow!("the stdin reader thread panicked: {panic:?}"))?
        .context("the stdin reader thread returned an error")?;

    Ok(())
}
