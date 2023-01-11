#!/usr/bin/env python3

import argparse
import sys

import requests


def main(argv=None):
    if argv is None:
        argv = sys.argv

    parser = argparse.ArgumentParser()

    parser.add_argument('bot_token', metavar='BOT_TOKEN', type=str)
    parser.add_argument('chat_id', metavar='CHAT_ID', type=str)

    args = vars(parser.parse_args(argv[1:]))

    bot_token = args['bot_token']
    chat_id = args['chat_id']

    ############################################################################

    bot_token = bot_token.removeprefix('bot')

    text = sys.stdin.read()

    text = text.strip()
    if text == '':
        print('Message must be non-empty', file=sys.stderr)
        return 1

    resp = requests.post(
        'https://api.telegram.org/bot%s/sendMessage' % bot_token,
        json={'chat_id': chat_id, 'text': text},
    )

    resptext = resp.text
    if not resptext.endswith('\n'):
        resptext += '\n'

    try:
        resp.raise_for_status()
        sys.stdout.write(resptext)
    except requests.exceptions.HTTPError:
        sys.stderr.write(resptext)
        raise

    return 0


if __name__ == '__main__':
    sys.exit(main())
