#!/usr/bin/env bash

set -e

cd $(dirname ${BASH_SOURCE[0]})/../..


while getopts p flag
do
    case "${flag}" in
        p) purge=true;;
    esac
done

if [ "$purge" = true ] ; then
./target/release/gateway purge-chain --base-path /tmp/chainz/charlie --chain ./gatewayChainSpec.json --database paritydb -y
fi

export ETH_KEY_ID=my_eth_key_id
export ETH_RPC_URL=https://ropsten-eth.compound.finance
export MINER="ETH:0x66613A2d4908D130C908ccF2f298b235bACD427a"
./target/release/gateway \
  --base-path /tmp/chainz/charlie \
  --chain ./gatewayChainSpec.json \
  --port 30335 \
  --ws-port 9947 \
  --rpc-port 9935 \
  --telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
  --validator \
  --no-mdns \
  --charlie \
  --rpc-methods Unsafe \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWQoFU7AqYZE5cCFdHVHgN4M25dnwZJEjUihV4FD3UmZhZ