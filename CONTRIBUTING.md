
# Contributing

When contributing to Gateway, please first discuss the changes you would like to make in [Discord](https://discord.gg/wMNpCBN4), on the community [forums](https://www.comp.xyz/) or by making a new Issue. We will generally not accept changes to this repository that have not been previously discussed and agreed upon by the community and other contributors.

Note we have a code of conduct; please follow it in all your interactions with the project.

## Release Process

All changes should be made to the main `develop` branch. Every so often a release will be cut from the develop branch for the next release (e.g. m10, m11, etc). These new releases will be proposed to governance for upgrade to test-net and eventually main-net.

Because of this, changes should be kept to a minimum and discussed before making a pull request. Changes that are accepted may still be kept in a holding pattern until after certain other patches have been released. For example, a new Starport may be ready to merge, but held until m33 and m34 have been released, in which case the new Starport change could be its own m35 release. These topics will be discussed in the pull request and milestones.

**The best way to get a change accepted as quickly as possible is to discuss with the community early and often**.

Finally, for larger changes, it may be advised that an audit is conducted for the changes. Please discuss in [Discord](https://discord.gg/wMNpCBN4) or the [forums](https://www.comp.xyz/) for recommended auditors.

## Development Process

When developing a change for Gateway, you will need to ensure the change is accurate, accomplishes its goal with the simplest mechanisms, and will successfully upgrade without disruption. You should:

1. Discuss your proposed patch in [Discord](https://discord.gg/wMNpCBN4), the [forums](https://www.comp.xyz/) or GitHub Issues.
2. Fork this repository.
3. Make your changes. Write clear code that is obvious in its effects.
4. Ensure that the current test cases pass and you've added new test cases for your patch.
5. Create a pull request for your change against the `develop` branch.
6. Your pull request, if acceptable, will be assigned to a milestone.
7. Once your pull request is merged, governance will be given the option of voting on the change.
8. If governance ascents, your patch will be upgraded on test-net and then main-net.

## Testing a Patch

There are several types of test-cases in this repository. It's mandatory that all patches pass all existing test cases and introduce test cases for its own changes.

### Rust Unit Tests

These tests are run via `cargo test`. This command runs all Substrate tests and other cargo package tests. The goal of these tests are to flex all lines of code and the goal is to approach about 100% code coverage.

Additionally, you will need to run `cargo bench` to run benchmarks. You should make sure to benchmark the weights of all new or significantly-modified extrinsics.

### Integration Tests

Gateway includes a homegrown variety of integration tests, sometimes called scenario tests. These are a Jest-based JavaScript framework which runs native instances of `gateway` nodes and externally interacts with the local test-net. These give high assurance in the effect of the code and patches to the code. It is integral that all changes properly integrate with these integration tests. See `/integration/README.md`
for more information.

### Chain Tests

Each L1 Chain, e.g. Ethereum, will have its own tests. These tests should be run and any changes to the L1 code should be unit tested, as well as being tested via integration tests.

## Soft versus Hard Upgrades

Your patch will be an upgrade to the Gateway chain. **It is strongly preferred that changes are soft-fork changes, if at all possible**. Specifically, Gateway includes a mechanism for runtime upgrades via governance (where the validator nodes do not need to re-compile and re-run processes for the upgrade). Certain changes will be incompatible with soft-fork upgrades (e.g. if you need a new native code integration for say, a different elliptic curve for digital signing). These changes are called hard-forks and will receive extra scrutiny. The following includes a list of the types of changes that may exist in soft-forks or hard-forks. This list is non-exhaustive and maintainers will scrutinize all patches for upgradability. Note: you will need to create an "upgrade" scenario to test how upgrades will work via a new integration test to prove the soft-fork or hard-fork upgrade.

### Soft Fork Patches

The following changes are generally considered safe for soft-fork patching:

1. Appending to exposed enum types (e.g. appending to the `ChainAsset` type).
2. Adding new storage items or appropriately modifying existing storage items via `on_runtime_upgrade`.
3. Adding or changing event types.
4. Adding or changing the `Reason` enum.
5. Adding new extrinsics.
6. Adding new RPCs.

### Hard Fork Patches

The following changes will likely force your patch to be introduced as a hard-fork upgrade. These are to be avoided if possible.

1. Introducing a new runtime interface (i.e. wasm host function).
2. Changing an existing extrinsic interface.
3. Changing an existing RPC interface.
4. Changing an exposed enum type (e.g. re-ordering the arms of `ChainAsset`).
5. Changing an existing storage item.
6. Adding new pallets.

## Code of Conduct

We expect all contributors to treat all community members with respect. We expect a community that is open to change and is free from personal interest. Failure to follow this policy may result in being removed from this repository. All changes are subject to the license contained in the root of this repository.

## Non-Canonical Development

This core repository is meant to be a good place for developing Gateway patches. However, true governance lies in the core [Governmence](https://compound.finance/governance) system. You may fork this repository and propose new changes to the system directly. If it's accepted by the governors of the protocol, this repository will pull in the change to mirror the on-chain state.

## Questions

Questions about this policy or development are best addressed in [Discord](https://discord.gg/wMNpCBN4), the [forums](https://www.comp.xyz/) or in Issues.
