#!/bin/bash

set -e

cd "$(dirname "$0")"

rm -fv helpers/fake-notifier-status.txt

bash helpers/fake-tail.sh |
    cargo run -q -- -di1 -m10 -- bash helpers/fake-notifier.sh 2>&1 |
    tee output.txt

diff -s --color {expected,output}.txt
