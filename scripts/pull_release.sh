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

files=("gateway_runtime.compact.wasm" "gateway_runtime.checksum" "types.json" "rpc.json" "contracts.json")

for file in ${files[@]}; do
  echo "Retreiving ${file}..."
  set -x
  curl -L "${repo_url}/releases/download/${version}/${file}" > "$release_dir/${file}"
  set +x
done

echo "Pulled release $version"
