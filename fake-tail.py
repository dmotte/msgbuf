#!/usr/bin/env python3

import sys
import time


def main():
    time.sleep(0.5)

    print('Hi, this is sample log content')
    print()
    print('Let\'s wait some time')
    sys.stdout.flush()
    time.sleep(2)

    print('And now guess what? Some more time')
    sys.stdout.flush()
    time.sleep(3)

    print('A little more')
    sys.stdout.flush()
    time.sleep(2)

    # print('Ok, stop')
    print('Ok, stop', end='')
    sys.stdout.flush()


if __name__ == '__main__':
    sys.exit(main())
