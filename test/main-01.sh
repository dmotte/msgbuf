#!/bin/bash

set -e

cd "$(dirname "$0")"

export RUST_LOG=DEBUG

args=(-i100 -m10)

exec bash helpers/run-check-output.sh expected-01.txt "${args[@]}"
