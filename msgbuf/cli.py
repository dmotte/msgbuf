#!/usr/bin/env python3

import argparse
import json
import logging
import os
import subprocess
import sys
import threading
import time


class shr:
    '''
    Stuff shared between threads
    '''
    cfg = None

    lock = threading.Lock()
    # The lock is required to access the following variables
    buffer = ''
    eof_stdin = False  # To notify the dequeuer when stdin finishes


def task_dequeuer():
    '''
    This thread reads ("dequeues") text chunks from the buffer and invokes the
    notifier command accordingly.
    '''
    while True:
        with shr.lock:
            msg = shr.buffer[:shr.cfg.maxlen]

            if msg != '':
                try:
                    logging.debug('Calling subprocess with input: ' +
                                  json.dumps(msg))
                    subprocess.run(
                        shr.cfg.notifier,
                        input=msg.encode(),
                        check=True,
                    )

                    shr.buffer = shr.buffer[len(msg):]
                    if shr.cfg.buf_file is not None:
                        with open(shr.cfg.buf_file, 'w') as f:
                            f.write(shr.buffer)
                except subprocess.CalledProcessError as e:
                    logging.warning('The notifier command returned non-zero '
                                    f'exit status {e.returncode}')

            if shr.buffer == '' and shr.eof_stdin:
                break

        time.sleep(shr.cfg.interval)


def main(argv=None):
    if argv is None:
        argv = sys.argv

    parser = argparse.ArgumentParser()

    parser.add_argument('notifier', metavar='NOTIFIER',
                        nargs=argparse.REMAINDER,
                        help='Notifier command. It will be invoked whenever a '
                        'message needs to be sent. The text will be passed to '
                        'the subprocess\'s stdin')

    parser.add_argument('-l', '--logging-level', type=str, default='INFO',
                        help='Logging level. Possible values: CRITICAL, ERROR, '
                        'WARNING, INFO, DEBUG (default: INFO)')
    parser.add_argument('-i', '--interval', type=int, default=60,
                        help='Interval (in seconds) between checks for '
                        'messages to be sent (default: 60)')
    parser.add_argument('-m', '--maxlen', type=int, default=1024,
                        help='Maximum message length in bytes (default: 1024)')
    parser.add_argument('-f', '--buf-file', type=str,
                        help='Path to the file to use as persistence for the '
                        'buffer. If omitted, no file will be used')

    shr.cfg = parser.parse_args(argv[1:])

    ############################################################################

    if not shr.cfg.notifier:
        print('Notifier command cannot be empty', file=sys.stderr)
        return 1

    try:
        shr.cfg.logging_level = {
            'CRITICAL': logging.CRITICAL,
            'ERROR': logging.ERROR,
            'WARNING': logging.WARNING,
            'INFO': logging.INFO,
            'DEBUG': logging.DEBUG,
        }[shr.cfg.logging_level]
    except:
        print('Invalid logging level specified', file=sys.stderr)
        return 1

    if shr.cfg.interval <= 0:
        print('Interval must be positive', file=sys.stderr)
        return 1

    if shr.cfg.maxlen <= 0:
        print('Maximum message length must be positive', file=sys.stderr)
        return 1

    ############################################################################

    logging.basicConfig(level=shr.cfg.logging_level,
                        format='%(asctime)s %(levelname)s %(message)s')

    logging.debug('Configuration: ' + str(shr.cfg))

    ############################################################################

    if shr.cfg.buf_file is not None:
        if os.path.exists(shr.cfg.buf_file):
            with open(shr.cfg.buf_file, 'r') as f:
                shr.buffer = f.read()
            logging.info('Read buffer content from file')
        else:
            with open(shr.cfg.buf_file, 'w') as f:
                pass
            logging.info('Buffer file created (did not exist yet)')

    ############################################################################

    t_dequeuer = threading.Thread(target=task_dequeuer, daemon=True)
    t_dequeuer.start()

    for line in sys.stdin:
        logging.debug('Read line: ' + json.dumps(line))
        with shr.lock:
            shr.buffer += line
            if shr.cfg.buf_file is not None:
                with open(shr.cfg.buf_file, 'w') as f:
                    f.write(shr.buffer)

    with shr.lock:
        shr.eof_stdin = True
    logging.info('Reached stdin EOF. Waiting for dequeuer to exit')
    t_dequeuer.join()
    logging.info('Dequeuer thread exited')

    return 0
