#!/bin/bash

set -e

cd "$(dirname "$0")"

export RUST_LOG=DEBUG

args=(-i100 -m10 -s10)

exec bash helpers/run-check-output.sh expected-02.txt "${args[@]}"
