#!/usr/bin/env bash

set -eo pipefail

cd $(dirname ${BASH_SOURCE[0]})/..

types_json="./types.json"

cp "$types_json" "$types_json.bak"

projects=("gateway" "pallet-cash" "pallet-oracle" "ethereum-client" "gateway-crypto")

set -x

json_files=(./base_types.json)

for project in ${projects[@]}; do
  json_file="$(mktemp)"
  echo "Building $project to $json_file"
  cargo clean -p $project
  TYPES_FILE="$json_file" cargo build -p $project
  json_files[${#json_files[@]}]="$json_file"
done

jq -s 'add' ${json_files[@]} | jq -r 'to_entries|sort|from_entries' > $types_json

echo "Built $types_json"
