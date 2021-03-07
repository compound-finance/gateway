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
  ./target/release/gateway build-spec --disable-default-bootnode --chain staging > gatewayChainSpec.json
  ./target/release/gateway purge-chain --base-path /tmp/chainz/alice --chain gatewayChainSpec.json --database paritydb -y
fi

if [ "$purge" = true ] ; then
  ./target/release/gateway purge-chain --base-path /tmp/chainz/alice --chain ./gatewayChainSpec.json --database paritydb -y
  ./target/release/gateway purge-chain --base-path /tmp/chainz/alice --chain local --database paritydb -y
fi
export ETH_KEY_ID=my_eth_key_id
export ETH_RPC_URL=https://ropsten-eth.compound.finance
export MINER="ETH:0x55413A2d4908D130C908ccF2f298b235bACD427a"
./target/release/gateway \
  --base-path /tmp/chainz/alice \
  --chain ./gatewayChainSpec.json \
  --port 30333 \
  --ws-port 9944 \
  --rpc-port 9933 \
  --no-mdns \
  --rpc-methods Unsafe \
  --telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
  --validator \
  --alice