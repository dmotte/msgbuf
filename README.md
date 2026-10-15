# msgbuf

[![GitHub main workflow](https://img.shields.io/github/actions/workflow/status/dmotte/msgbuf/main.yml?branch=main&logo=github&label=main&style=flat-square)](https://github.com/dmotte/msgbuf/actions)
[![GitHub (Pre-)Release Date](https://img.shields.io/github/release-date-pre/dmotte/msgbuf?logo=github&style=flat-square)](https://github.com/dmotte/msgbuf/releases/latest)
[![GitHub all releases](https://img.shields.io/github/downloads/dmotte/msgbuf/total?logo=github&style=flat-square)](https://github.com/dmotte/msgbuf/releases/latest)

:crab: A standalone **message buffer** application.

This tool reads text from the **standard input** (`stdin`) stream and divides it into chunks, called "**messages**". For each message, a **notifier** command is invoked concurrently.

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
    processTail(#9881;#65039; <code>tail -f</code> #128220;)
    processCurl(#9881;#65039; <code>curl</code> #9992;#65039;)

    threadStdinReader(#129525; Thread <code>stdin-reader</code>)
    threadMain(#129525; Thread <code>main</code>)

    vecBuffer[#128203; Buffer]

    %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%

    processTail --> threadStdinReader

    subgraph msgbuf [#9881;#65039; <code>msgbuf</code>]
        threadStdinReader -- channel --> threadMain

        threadMain -- put --> vecBuffer -- get --> threadMain
    end

    threadMain --> processCurl
```

> **Note**: for more information on how to use this tool, you can also refer to the help message (`--help`).

## Implementation

Below is a **flowchart** of the logic of the `App.run` function:

```mermaid
flowchart TD
    Start([#127937; <b>START</b>]) --> recvBlocking

    recvBlocking["#128229; recv (blocking)"]
    recvBlocking -- Ok --> addByteBlocking
    recvBlocking -- EOF --> chkLenAll

    addByteBlocking[#10133; add_byte] --> chkFullBlocking

    chkFullBlocking{"#10067; len_all == buf.len()"}
    chkFullBlocking -- "true (buf is full)" --> sleepSendLenAllAsFull
    chkFullBlocking -- "false (buf is NOT full)" --> chkLenSend

    sleepSendLenAllAsFull[#128164; sleep_for_send] --> sendLenAllAsFull

    sendLenAllAsFull["#9992;#65039; try_send(len_all)"]
    sendLenAllAsFull -- true --> recvBlocking
    sendLenAllAsFull -- false --> sleepSendLenAllAsFull

    chkLenSend{"#10067; len_send > 0"}
    chkLenSend -- "true (could send something)" --> sleepMinRecvTime
    chkLenSend -- "false (cannot send anything)" --> recvBlocking

    sleepMinRecvTime[#128164; sleep_for_recv] --> sleepRecvPolling

    sleepRecvPolling[#128164; sleep_for_send] --> recvPolling

    recvPolling[#128229; try_recv]
    recvPolling -- Ok --> addBytePolling
    recvPolling -- Empty --> sendLenSendAsPollingEmpty
    recvPolling -- EOF --> chkLenAll

    addBytePolling[#10133; add_byte] --> chkFullPolling

    chkFullPolling{"#10067; len_all == buf.len()"}
    chkFullPolling -- "true (buf is full)" --> sendLenSendAsFull
    chkFullPolling -- "false (buf is NOT full)" --> recvPolling

    sendLenSendAsFull["#9992;#65039; try_send(len_send)"]
    sendLenSendAsFull -- true --> recvBlocking
    sendLenSendAsFull -- false --> sleepSendLenSendAsFull

    sleepSendLenSendAsFull[#128164; sleep_for_send] --> sendLenSendAsFull

    sendLenSendAsPollingEmpty["#9992;#65039; try_send(len_send)"]
    sendLenSendAsPollingEmpty -- true --> recvBlocking
    sendLenSendAsPollingEmpty -- false --> sleepMinRecvTime

    chkLenAll{"#10067; len_all > 0"}
    chkLenAll -- "true (something in buf)" --> sleepSendLenAllAsEOF
    chkLenAll -- "false (nothing in buf)" --> End

    sleepSendLenAllAsEOF[#128164; sleep_for_send] --> sendLenAllAsEOF

    sendLenAllAsEOF["#9992;#65039; try_send(len_all)"]
    sendLenAllAsEOF -- true --> End
    sendLenAllAsEOF -- false --> sleepSendLenAllAsEOF

    End([#127937; <b>END</b>])
```

## TODO

Consider adding some more end-to-end tests to cover more cases?

Consider writing some unit tests.
