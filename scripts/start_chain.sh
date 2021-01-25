#!/usr/bin/env bash

set -e

cd $(dirname ${BASH_SOURCE[0]})/..

cargo build --release
./target/release/compound-chain build-spec --disable-default-bootnode --chain staging > compoundChainSpec.json
./target/release/compound-chain build-spec --chain=compoundChainSpec.json --raw --disable-default-bootnode > compoundChainSpecRaw.json

./target/release/compound-chain purge-chain --base-path /tmp/chainz/alice --chain local --database paritydb
./target/release/compound-chain \
  --base-path /tmp/chainz/alice \
  --chain ./compoundChainSpecRaw.json \
  --alice \
  --port 30333 \
  --ws-port 9944 \
  --rpc-port 9933 \
  --telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
  --validator \
  --no-mdns

./target/release/compound-chain purge-chain --base-path /tmp/chainz/bob --chain ./compoundChainSpecRaw.json --database paritydb
./target/release/compound-chain \
  --base-path /tmp/chainz/bob \
  --chain ./compoundChainSpecRaw.json \
  --bob \
  --port 30334 \
  --ws-port 9946 \
  --rpc-port 9934 \
  --telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
  --validator \
  --no-mdns \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp

./target/release/compound-chain purge-chain --base-path /tmp/chainz/charlie --chain ./compoundChainSpecRaw.json --database paritydb
./target/release/compound-chain \
  --base-path /tmp/chainz/charlie \
  --chain ./compoundChainSpecRaw.json \
  --charlie \
  --port 30335 \
  --ws-port 9947 \
  --rpc-port 9935 \
  --telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
  --validator \
  --no-mdns \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp