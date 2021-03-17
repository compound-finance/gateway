#!/usr/bin/env bash

set -e

cd $(dirname ${BASH_SOURCE[0]})/../..

while getopts "a:pbc:" flag
do
    case "${flag}" in
        a) actor=$OPTARG;;
        p) purge=true;;
        b) build=true;;
        c) chain=$OPTARG;;
        v) validator=true;;
    esac
done

case "$actor" in
  "alice")
    export ETH_KEY_ID=my_eth_key_id
    export ETH_RPC_URL=https://ropsten-eth.compound.finance
    export MINER="ETH:0x55413A2d4908D130C908ccF2f298b235bACD427a"
    export port=30333
    export wsPort=9944
    export rpcPort=9933
    if [ "$validator" == true ]; then
      gatewayArgs="--alice"
    fi
    ;;

  "bob")
    export ETH_KEY_ID=my_eth_key_id
    export ETH_RPC_URL=https://ropsten-eth.compound.finance
    export MINER="ETH:0x66613A2d4908D130C908ccF2f298b235bACD427a"
    export port=30334
    export wsPort=9945
    export rpcPort=9934
    if [ "$validator" == true ]; then
      gatewayArgs="--bob"
    fi
    ;;

  "charlie")
    export ETH_KEY_ID=my_eth_key_id
    export ETH_RPC_URL=https://ropsten-eth.compound.finance
    export MINER="ETH:0x66613A2d4908D130C908ccF2f298b235bACD427a"
    export port=30335
    export wsPort=9946
    export rpcPort=9934
    if [ "$validator" == true ]; then
      gatewayArgs="--charlie"
    fi
    ;;

  "")
    echo "Please set actor arg with -a \$actor"
    exit 1
    ;;

  *)
    echo "Unknown actor: \"$actor\""
    exit 1
    ;;
esac

if [ "$build" = true ] ; then
  cargo build --release
fi

if [ -z "$chain" ]; then
  chain="testnet"
fi

chainFile=./chains/$chain/chain-spec-raw.json
basePath="$(mktemp -d)"

if [ ! -f "$chainFile" ]; then
  echo "Cannot find chain $chain at $chainFile. Try running \"chains/build_spec.js $chain\""
fi

if [ "$purge" = true ] ; then
   ./target/release/gateway purge-chain --base-path "$basePath" --chain "$chainFile" --database paritydb -y
   ./target/release/gateway purge-chain --base-path "$basePath" --chain local --database paritydb -y
fi

set -x

./target/release/gateway \
  --base-path "$basePath" \
  --chain "$chainFile" \
  --port "$port" \
  --ws-port "$wsPort" \
  --rpc-port "$rpcPort" \
  --no-mdns \
  --rpc-methods Unsafe \
  --validator \
  -lbasic_authorship=trace,txpool=trace,afg=trace,aura=trace \
  $gatewayArgs
