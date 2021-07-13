#!/usr/bin/env bash

set -eo pipefail

version="$1"
machine=`node -e 'console.log(process.platform + "-" + process.arch)'`

if [ -z "$version" ]; then
	echo ""
	echo "usage: scripts/build_release {version}"
	echo ""
	exit 1
fi

echo "*** Building release $version on $machine ***"

cd $(dirname ${BASH_SOURCE[0]})/..

release_dir="./releases/$version"

if [ -d "$release_dir" ]; then
	if [ -n "$CLEAN" ]; then
		rm -rf "$release_dir"
	else
		echo "Release directory $release_dir already exists. Please remove before building release or run with CLEAN=true."
		exit 1;
	fi
fi

echo "*** Building Solidity ***"

(cd ethereum && yarn install && yarn compile)

contracts="./ethereum/.build/contracts.json"

echo "*** Building release gateway ***"

cargo +nightly build --release --features runtime-debug

bin="./target/release/gateway"
types="./types.json"
rpc="./rpc.json"
wasm="./target/release/wbuild/gateway-runtime/gateway_runtime.compact.wasm"

if [ ! -f "$bin" -o ! -f "$wasm" -o ! -f "$types" -o ! -f "$rpc" -o ! -f "$contracts" ]; then
	echo "Missing one of the following build files: $bin, $wasm, $types, $rpc, $contracts"
	exit 1
fi

echo "*** Building checksums ***"
wasm_checksum="$(node ./ethereum/scripts/utils/keccak.js "$wasm")"
bin_checksum="$(node ./ethereum/scripts/utils/keccak.js "$bin")"

mkdir -p "$release_dir"

cp "$wasm" "$release_dir/gateway.wasm"
echo "$wasm_checksum" > "$release_dir/gateway.wasm.checksum"

cp "$bin" "$release_dir/gateway-$machine"
echo "$bin_checksum" > "$release_dir/gateway-$machine.checksum"

cp "$types" "$release_dir/types.json"
cp "$rpc" "$release_dir/rpc.json"
cp "$contracts" "$release_dir/contracts.json"

echo "Built release $version"
echo "  wasm: $release_dir/gateway.wasm"
echo "  wasm.checksum: $release_dir/gateway.wasm.checksum"
echo "  bin: $release_dir/gateway-$machine"
echo "  bin.checksum: $release_dir/gateway-$machine.checksum"
echo "  types: $release_dir/types.json"
echo "  rpc: $release_dir/rpc.json"
echo "  contracts: $release_dir/contracts.json"
