#!/usr/bin/env bash

set -eo pipefail

cargo build --release --features runtime-benchmarks

target/release/gateway benchmark \
    --execution wasm \
    --wasm-execution compiled \
    --pallet pallet_cash \
    --extrinsic '*' \
    --steps 10 \
    --repeat 10 \
    --raw \
    --template=./.maintain/frame-weight-template.hbs \
    --output=./pallets/cash/src/weights.rs