#!/usr/bin/env bash

set -eo pipefail

cd $(dirname ${BASH_SOURCE[0]})/..

types_json="./types.json"

cp "$types_json" "$types_json.bak"

projects=("pallet-cash" "pallet-oracle" "ethereum-client")

set -x

json_files=()

for project in ${projects[@]}; do
  json_file="$(mktemp)"
  echo "Building $project to $json_file"
  cargo clean -p $project
  TYPES_FILE="$json_file" cargo build -p $project
  json_files[${#json_files[@]}]="$json_file"
done

jq -sS 'add' ${json_files[@]} > $types_json

echo "Built $types_json"
