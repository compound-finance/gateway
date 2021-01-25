#!/usr/bin/env bash

set -e

echo "*** Building Chain Spec from staging ***"

cd $(dirname ${BASH_SOURCE[0]})/..

cargo build --release
./target/release/compound-chain build-spec --disable-default-bootnode --chain staging > compoundChainSpec.json
./target/release/compound-chain build-spec --chain=compoundChainSpec.json --raw --disable-default-bootnode > compoundChainSpecRaw.json