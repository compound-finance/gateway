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
