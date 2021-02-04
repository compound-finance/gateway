#!/usr/bin/env bash

set -e

cd $(dirname ${BASH_SOURCE[0]})/../..

while getopts pb flag
do
    case "${flag}" in
        p) purge=true;;
        b) build=true;;
    esac
done

if [ "$build" = true ] ; then
  cargo build --release
  ./target/release/compound-chain build-spec --disable-default-bootnode --chain staging > compoundChainSpec.json
  ./target/release/compound-chain purge-chain --base-path /tmp/chainz/alice --chain compoundChainSpec.json --database paritydb -y
fi

if [ "$purge" = true ] ; then
  ./target/release/compound-chain purge-chain --base-path /tmp/chainz/alice --chain ./compoundChainSpec.json --database paritydb -y
fi

./target/release/compound-chain \
  --base-path /tmp/chainz/alice \
  --chain ./compoundChainSpec.json \
  --port 30333 \
  --ws-port 9944 \
  --rpc-port 9933 \
  --no-mdns \
  --rpc-methods Unsafe \
  --telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
  --validator