#!/usr/bin/env bash

set -e

echo "*** Building Chain Spec from staging ***"

cd $(dirname ${BASH_SOURCE[0]})/..

cargo build --release
./target/release/gateway build-spec --disable-default-bootnode --chain staging > alphaTestnetChainSpec.json
#TODO: just put it in chain_spec.rs...
#TODO: set liquidity factors in script
#XXX liquidty fators et al are off by several decimal places, since jq was converting them to scientific notation and breaking when hitting rust,
cat alphaTestnetChainSpec.json | jq --slurpfile asset_config ./scripts/chains/ropsten_token_config.json 'setpath(["genesis", "runtime", "palletCash", "assets"]; $asset_config)' > m.json
# here, stop and add the needed decimals to the m.json ( or do something smarter )
./target/release/gateway build-spec --chain=m.json --raw --disable-default-bootnode > alphaTestnetChainSpecRaw.json
# rm m.json