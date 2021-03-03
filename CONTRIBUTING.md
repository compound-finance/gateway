
# TODO: Start to fill out a full guide here

## Upgrade Guidelines

The following changes are considered safe for patching:

1. Adding new types (with corresponding additions to `types.json`
2. Appending to enum types
3. Adding new storage items
4. Adding or changing event types
5. Adding extrinsics
6. Adding new RPCs


The following changes are considered safe for minor upgrades:

1. Changing a type with a corresponding change in `types.json`
2. Changing extrinsics
3. Changing the _interface_ to storage items or RPCs

The following are considered major upgrades:

1. Changing the underlying storage mechanisms

