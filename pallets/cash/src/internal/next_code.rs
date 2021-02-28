use frame_support::{storage::StorageValue, traits::UnfilteredDispatchable};

use crate::{
    chains::{Chain, Ethereum},
    reason::Reason,
    require,
    types::CodeHash,
    AllowedNextCodeHash, Config, Event, Module,
};

pub fn allow_next_code_with_hash<T: Config>(hash: CodeHash) -> Result<(), Reason> {
    AllowedNextCodeHash::put(hash);
    <Module<T>>::deposit_event(Event::AllowedNextCodeHash(hash));
    Ok(())
}

pub fn set_next_code_via_hash<T: Config>(code: Vec<u8>) -> Result<(), Reason> {
    // XXX what Hash type to use?
    let hash = <Ethereum as Chain>::hash_bytes(&code);
    require!(
        Some(hash) == AllowedNextCodeHash::get(),
        Reason::InvalidCodeHash
    );
    AllowedNextCodeHash::kill();
    let call = frame_system::Call::<T>::set_code(code);
    let result = call.dispatch_bypass_filter(frame_system::RawOrigin::Root.into());
    <Module<T>>::deposit_event(Event::AttemptedSetCodeByHash(
        hash,
        result.map(|_| ()).map_err(|e| e.error),
    ));
    Ok(())
}
