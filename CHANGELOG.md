# GATE Addresses [m12]

This release is focused on the ability to send assets to GATE addresses. This allows us to create accounts past "existential deposits" and therefore allows validators to set their session keys again.

* [Feature] Added ability to send assets to GATE addresses

# New Event Ingression System [m11]

This release introduces a new event ingression system. This system properly handles re-organizations of the Ethereum chain. Additionally, the system introduces a risk system to ingress high-value lock events with an exponential decay.

We additionally introduce a REPL to connect a shell to test-net and make a variety of internal improvements to the internal code. We continue to add greater test coverage.

* [Feature] New Event Ingression System with Re-org Protection
* [Feature] Add repl to connect shell to test-net
* [Feature] Properly add weights to all extrinsics with benchmarks
* [Feature] Auto-derive types.json via rust macros
* [Feature] Added new RPCs for retrieving detailed account information
* [Feature] Add rpc.json to decorate RPC calls
* [Bug] Fix logic issues in core liquidation code
* [Internal] Refactor of core functions using "pipeline" system
* [Testing] Add ability for tests to "freeze time" for accurate interest testing
* [Testing] Improve event tracking between version upgrades
* [Feature] Change from environment variables to CLI args
