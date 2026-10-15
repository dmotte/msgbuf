#!/bin/bash

set -e

expected_txt=${1:?}; shift

expected_txt=$(realpath "$expected_txt")

cd "$(dirname "$0")"

################################################################################

rm -fv fake-notifier-status.txt

bash fake-tail.sh |
    cargo run -q -- "$@" -- bash fake-notifier.sh 2>&1 |
    tee output.txt

diff -s --color "$expected_txt" output.txt
