#!/usr/bin/env bash

set -e

echo "*** Building Chain Spec from staging ***"

cd $(dirname ${BASH_SOURCE[0]})/..

cargo build --release
./target/release/compound-chain build-spec --disable-default-bootnode --chain staging > alphaTestnetChainSpec.json
#todo: just put it in chain_spec.rs...
cat alphaTestnetChainSpec.json | jq --slurpfile ass ./scripts/chains/ropsten_token_config.json 'setpath(["genesis", "runtime", "palletCash", "assets"]; $ass)' > m.json
./target/release/compound-chain build-spec --chain=m.json --raw --disable-default-bootnode > alphaTestnetChainSpecRaw.json
rm m.json