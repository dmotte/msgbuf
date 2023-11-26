#!/bin/bash

set -e

cd "$(dirname "$0")"

rm -f fake-notifier-status.txt

bash fake-tail.sh | \
    cargo run -q -- -di1 -m10 -- bash fake-notifier.sh 2>&1 | \
    tee output.txt

diff {expected,output}.txt
