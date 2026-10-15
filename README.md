# msgbuf

[![GitHub main workflow](https://img.shields.io/github/actions/workflow/status/dmotte/msgbuf/main.yml?branch=main&logo=github&label=main&style=flat-square)](https://github.com/dmotte/msgbuf/actions)
[![GitHub (Pre-)Release Date](https://img.shields.io/github/release-date-pre/dmotte/msgbuf?logo=github&style=flat-square)](https://github.com/dmotte/msgbuf/releases/latest)
[![GitHub all releases](https://img.shields.io/github/downloads/dmotte/msgbuf/total?logo=github&style=flat-square)](https://github.com/dmotte/msgbuf/releases/latest)

:crab: A standalone **message buffer** application.

This tool reads text from the **standard input** (`stdin`) stream and divides it into chunks, called "**messages**". Then, for each message, a **notifier** command is invoked on a separate thread.

## Installation

Pre-built binaries are available. You can download the latest version with:

```bash
curl -fLo msgbuf "https://github.com/dmotte/msgbuf/releases/latest/download/msgbuf-$(curl -fsSL https://sh.rustup.rs/ | RUSTUP_INIT_SH_PRINT=arch bash)"
chmod -v +x msgbuf
```

Or simpler approach (but you need to know the `-<vendor>-<sys>-<abi>` part of the Rust **target triple**):

```bash
curl -fLo msgbuf "https://github.com/dmotte/msgbuf/releases/latest/download/msgbuf-$(uname -m)-unknown-linux-gnu"
chmod -v +x msgbuf
```

## Usage

As a sample notifier command we can use _cURL_ to send the text to a _Telegram_ user (as a bot). Example:

```bash
tail -f /var/log/my-log-file | ./msgbuf -- curl -sSXPOST "https://api.telegram.org/bot.../sendMessage" -dchat_id=... --data-urlencode text@- --fail-with-body -w'\n'
```

This is a simple graphical representation of what happens when we run the shell line above:

```mermaid
graph LR
    tail(fa:fa-gear tail -f fa:fa-scroll)
    reader(fa:fa-gear reader)
    invoker(fa:fa-gear invoker)
    curl(fa:fa-gear curl fa:fa-paper-plane)

    tail --> reader

    subgraph fa:fa-gear msgbuf
        reader -- "channel" --> invoker

        invoker -- "put" --> msg
        msg -- "get" --> invoker
    end

    invoker --> curl
```

> **Note**: for more information on how to use this tool, you can also refer to the help message (`--help`).

TODO recheck everything (especially README and end-to-end tests) to follow the new implementation

TODO consider adding some more end-to-end tests to cover more cases?

TODO try to fix `App.run` could run on an unclean struct. Maybe use a static function with variable move

TODO consider writing some unit tests

TODO flowchart draft:

```mermaid
flowchart TD
    Start([START])
    Start --> recvBlocking

    recvBlocking["recv (blocking)"]
    recvBlocking -- Ok --> addByteBlocking[add_byte] --> chkFullBlocking
    recvBlocking -- EOF --> chkLenAll

    chkFullBlocking{"len_all == buf.len() ?"}
    chkFullBlocking -- "true (buf is full)" --> sleepSendLenAllAsFull
    chkFullBlocking -- "false (buf is NOT full)" --> chkLenSend

    sleepSendLenAllAsFull[sleep_for_send]
    sleepSendLenAllAsFull --> sendLenAllAsFull

    sendLenAllAsFull["try_send(len_all)"]
    sendLenAllAsFull -- true --> recvBlocking
    sendLenAllAsFull -- false --> sleepSendLenAllAsFull

    chkLenSend{"len_send > 0 ?"}
    chkLenSend -- "true (could send something)" --> sleepMinRecvTime
    chkLenSend -- "false (cannot send anything)" --> recvBlocking

    sleepMinRecvTime[sleep_for_recv]
    sleepMinRecvTime --> sleepRecvPolling[sleep_for_send] --> recvPolling

    recvPolling[try_recv]
    recvPolling -- Ok --> addBytePolling[add_byte] --> chkFullPolling
    recvPolling -- Empty --> sendLenSendAsPollingEmpty
    recvPolling -- EOF --> chkLenAll

    chkFullPolling{"len_all == buf.len() ?"}
    chkFullPolling -- "true (buf is full)" --> sendLenSendAsFull
    chkFullPolling -- "false (buf is NOT full)" --> recvPolling

    sendLenSendAsFull["try_send(len_send)"]
    sendLenSendAsFull -- true --> recvBlocking
    sendLenSendAsFull -- false --> sleepSendLenSendAsFull

    sleepSendLenSendAsFull[sleep_for_send]
    sleepSendLenSendAsFull --> sendLenSendAsFull

    sendLenSendAsPollingEmpty["try_send(len_send)"]
    sendLenSendAsPollingEmpty -- true --> recvBlocking
    sendLenSendAsPollingEmpty -- false --> sleepMinRecvTime

    chkLenAll{"len_all > 0 ?"}
    chkLenAll -- "true (something in buf)" --> sleepSendLenAllAsEOF
    chkLenAll -- "false (nothing in buf)" --> End

    sleepSendLenAllAsEOF[sleep_for_send]
    sleepSendLenAllAsEOF --> sendLenAllAsEOF

    sendLenAllAsEOF["try_send(len_all)"]
    sendLenAllAsEOF -- true --> End
    sendLenAllAsEOF -- false --> sleepSendLenAllAsEOF

    End([END])
```
