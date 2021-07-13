#!/usr/bin/env bash

set -eo pipefail

version="$1"

if [ -z "$version" ]; then
  echo ""
  echo "usage: scripts/pull_release {version}"
  echo ""
  exit 1
fi

repo_url="${REPO_URL:-https://github.com/compound-finance/gateway}"

echo "*** Pulling release $version ***"

cd $(dirname ${BASH_SOURCE[0]})/..

release_dir="./releases/$version"

mkdir -p "$release_dir"

files=("contracts.json" "gateway-darwin-arm64" "gateway-darwin-arm64.checksum" "gateway-linux-x86" "gateway.wasm" "gateway.wasm.checksum" "rpc.json" "types.json")

for file in ${files[@]}; do
  echo "Retreiving ${file}..."
  set -x
  curl -L "${repo_url}/releases/download/${version}/${file}" > "$release_dir/${file}"
  set +x
done

echo "Pulled release $version"
