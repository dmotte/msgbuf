#!/usr/bin/env python3

import sys
import json
import random


def main():
    msg = sys.stdin.read()

    if random.random() < .5:
        #print('FAKE-NOTIFIER: Fake error', file=sys.stderr)
        return 1

    print('FAKE-NOTIFIER:', json.dumps(msg))

    return 0


if __name__ == '__main__':
    sys.exit(main())
