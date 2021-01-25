#!/usr/bin/env bash

set -e

cd $(dirname ${BASH_SOURCE[0]})/../..

./target/release/compound-chain purge-chain --base-path /tmp/chainz/charlie --chain ./compoundChainSpecRaw.json --database paritydb -y

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
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWDeCffMfq1caXTPZSy2nmNxRzAgLKtmRLxTu8FndFGv6n