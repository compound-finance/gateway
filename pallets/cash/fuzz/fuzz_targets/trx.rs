#![no_main]
use libfuzzer_sys::fuzz_target;
use std::str;
use pallet_cash::tests::*;

fuzz_target!(|data: &[u8]| {
    let v = str::from_utf8(data);
    new_test_ext().execute_with(|| {

    });
});

