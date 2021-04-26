#![no_main]
use std::str;
use libfuzzer_sys::fuzz_target;
use pallet_cash::chains::ChainAccount;
use std::str::FromStr;

fuzz_target!(|data: &[u8]| {
    let data = str::from_utf8(data);
    if let Ok(v) = data {
        ChainAccount::from_str(v);
    }
});
