#!/bin/sh

base=$(dirname "$0")

function run_host() {
  cargo run -p api_server -- \
    --mysql-pass 6434443248b2
}

(cd "$base/.." && run_host)
