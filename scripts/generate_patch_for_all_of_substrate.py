"""
Patch substrate with a local verison of substrate. Useful for fixing bugs in substrate while working on gateway.

PRECONDITIONS
* git folders are checked out in the same directory as in
* run this script from the gateway directory

USAGE
1. run
python3 ./scripts/generate_patch_for_all_of_substrate.py | pbcopy

2. Now you have the patch on your clipboard, paste in into the top level Cargo.toml file.
"""
import os
import toml

workspace = toml.load('../substrate/Cargo.toml')
paths = workspace['workspace']['members']
print('[patch."https://github.com/compound-finance/substrate.git"]')
for path in paths:
    candidate_path = os.path.join('../substrate', path)
    candidate_cargo_file = os.path.join(candidate_path, 'Cargo.toml')
    if os.path.exists(candidate_cargo_file):
        parsed = toml.load(candidate_cargo_file)
        package_name = parsed['package']['name']
        print(f'{package_name} = {{ path = "{candidate_path}" }}')
