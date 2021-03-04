#!/usr/bin/env bash

set -eo pipefail

version="$1"

if [ -z "$version" ]; then
	echo ""
	echo "usage: scripts/build_release {version}"
	echo ""
	exit 1
fi

echo "*** Building release $version ***"

cd $(dirname ${BASH_SOURCE[0]})/..

echo "*** Building Solidity ***"

(cd ethereum && yarn install && yarn compile)

contracts="./ethereum/.build/contracts.json"

echo "*** Building release compound chain ***"

cargo build --release

bin="./target/release/compound-chain"
types="./types.json"
wasm="./target/release/wbuild/compound-chain-runtime/compound_chain_runtime.compact.wasm"

if [ ! -f "$bin" -o ! -f "$wasm" -o ! -f "$types" -o ! -f "$contracts" ]; then
	echo "Missing one of the following build files: $bin, $wasm, $types, $contracts"
	exit 1
fi

echo "*** Building checksum of wasm ***"
checksum="$(node ./ethereum/scripts/utils/keccak.js "$wasm")"

release_dir="./releases/$version"

mkdir -p "$release_dir"
cp "$wasm" "$release_dir/compound_chain_runtime.compact.wasm"
echo "$checksum" > "$release_dir/compound_chain_runtime.checksum"
cp "$types" "$release_dir/types.json"
cp "$contracts" "$release_dir/contracts.json"

echo "Built release $version"
echo "  wasm: $release_dir/compound_chain_runtime.compact.wasm"
echo "  wasm.checksum: $release_dir/compound_chain_runtime.checksum"
echo "  types: $release_dir/types.json"
echo "  contracts: $release_dir/contracts.json"
