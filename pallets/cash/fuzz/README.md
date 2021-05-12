
To run the fuzzer locally, ensure you have the fuzz tool:

```bash
cargo +nightly install cargo-fuzz
```

Then run a fuzz target, e.g.:

```bash
(cd pallets/cash && RUST_BACKTRACE=full cargo +nightly fuzz run chain_account_from_str_fuzz)
```
