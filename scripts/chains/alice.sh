#!/usr/bin/env bash

set -e

cd $(dirname ${BASH_SOURCE[0]})/../..

cargo build --release
./target/release/compound-chain build-spec --disable-default-bootnode --chain local > compoundChainSpec.json
# ./target/release/compound-chain build-spec --chain=compoundChainSpec.json --raw --disable-default-bootnode > compoundChainSpecRaw.json

# ./target/release/compound-chain purge-chain --base-path /tmp/chainz/alice --chain compoundChainSpecRaw.json --database paritydb -y
./target/release/compound-chain purge-chain --base-path /tmp/chainz/alice --chain compoundChainSpec.json --database paritydb -y

./target/release/compound-chain \
  --base-path /tmp/chainz/alice \
  --chain ./compoundChainSpec.json \
  --alice \
  --port 30333 \
  --ws-port 9944 \
  --rpc-port 9933 \
  --rpc-methods Unsafe \
  --no-mdns \
  --telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
  --validator 
#   --chain ./compoundChainSpecRaw.json \