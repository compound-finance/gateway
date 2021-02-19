use frame_support::storage::StorageValue;

use crate::{
    chains::{Chain, Ethereum},
    reason::Reason,
    require,
    types::CodeHash,
    AllowedNextCodeHash, Config,
};

pub fn allow_next_code_with_hash<T: Config>(hash: CodeHash) -> Result<(), Reason> {
    AllowedNextCodeHash::put(hash);
    Ok(())
}

pub fn set_next_code_via_hash<T: Config>(code: Vec<u8>) -> Result<(), Reason> {
    // XXX what Hash type to use?
    require!(
        Some(<Ethereum as Chain>::hash_bytes(&code)) == AllowedNextCodeHash::get(),
        Reason::InvalidCodeHash
    );
    AllowedNextCodeHash::kill();
    // XXX
    //System::Call::<T>::set_code(frame_system::RawOrigin::root(), code);
    Ok(())
}
