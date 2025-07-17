use clap::Parser;
use std::io::{self, Read, Write};
use std::process::{Command, Stdio, exit};
use std::sync::mpsc::{self, RecvError, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

const CHAN_BUF_SIZE: usize = 4096;
const MIN_RECV_TIME: Duration = Duration::from_millis(100);

/// Message buffer
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable debug messages
    #[arg(short, long)]
    debug: bool,

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

fn main() {
    let args = Args::parse();
    if args.debug {
        println!("DEBUG: {args:?}");
    }
    let interval = Duration::from_secs(args.interval);
    if args.notifier.is_empty() {
        eprintln!("Notifier command cannot be empty");
        exit(1);
    }

    let (tx, rx) = mpsc::sync_channel(CHAN_BUF_SIZE);

    let hnd_read_and_send = thread::spawn(move || {
        for b in io::stdin().bytes() {
            tx.send(b.unwrap()).unwrap();
        }
    });

    let mut msg = vec![b'\0'; args.max_msg_len];
    let mut msg_len: usize = 0;
    let mut stdin_eof = false;
    let mut next_invocation = Instant::now();

    if args.debug {
        println!("DEBUG: recv_and_invoke loop starting");
    }

    'recv_and_invoke: loop {
        // Step 1: if msg is empty, start RECEIVING one byte (blocking)

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
            };

            // Give the sender some time to send everything it has to
            thread::sleep(MIN_RECV_TIME);
        }

        // Step 2: wait some time before proceeding, if needed

        let now = Instant::now();
        if next_invocation < now {
            // We're "behind schedule", so let's reset next_invocation
            next_invocation = now + interval;
        } else {
            thread::sleep(next_invocation - now);
            next_invocation += interval;
        }

        // Step 3: RECEIVE the rest, until msg is full or EOF is reached

        'nonblocking_recv: while !stdin_eof && msg_len < args.max_msg_len {
            // Note: the call to try_recv is non-blocking
            match rx.try_recv() {
                Ok(b) => {
                    msg[msg_len] = b;
                    msg_len += 1;
                }
                Err(TryRecvError::Empty) => break 'nonblocking_recv,
                Err(TryRecvError::Disconnected) => stdin_eof = true,
            };
        }

        if stdin_eof && msg_len == 0 {
            break 'recv_and_invoke;
        }

        // Step 4: INVOKE the notifier command

        if args.debug {
            println!(
                "DEBUG: invoking notifier with input: {:?}",
                std::str::from_utf8(&msg[0..msg_len]).unwrap()
            );
        }

        let mut child = Command::new(&args.notifier[0])
            .args(&args.notifier[1..])
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&msg[0..msg_len])
            .unwrap();
        let exit_status = child.wait().unwrap();
        if exit_status.success() {
            msg_len = 0;
        } else {
            eprintln!(
                "The notifier command returned non-zero exit status {}",
                exit_status.code().unwrap()
            );
        }

        if stdin_eof && msg_len == 0 {
            break 'recv_and_invoke;
        }
    }

    if args.debug {
        println!("DEBUG: recv_and_invoke loop finished");
    }

    hnd_read_and_send.join().unwrap();
}
