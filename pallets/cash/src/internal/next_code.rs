use crate::{
    chains::{Chain, Gateway},
    reason::Reason,
    require,
    types::CodeHash,
    AllowedNextCodeHash, Config, Event, Module,
};
use frame_support::{dispatch::DispatchResultWithPostInfo, storage::StorageValue};

pub fn allow_next_code_with_hash<T: Config>(hash: CodeHash) -> Result<(), Reason> {
    AllowedNextCodeHash::put(hash);
    <Module<T>>::deposit_event(Event::AllowedNextCodeHash(hash));
    Ok(())
}

#[cfg(not(test))]
fn dispatch_call<T: Config>(code: Vec<u8>) -> DispatchResultWithPostInfo {
    use frame_support::traits::UnfilteredDispatchable;
    let call = frame_system::Call::<T>::set_code(code);
    call.dispatch_bypass_filter(frame_system::RawOrigin::Root.into())
}

pub fn set_next_code_via_hash<T: Config>(code: Vec<u8>) -> Result<(), Reason> {
    let hash = <Gateway as Chain>::hash_bytes(&code);
    require!(
        Some(hash) == AllowedNextCodeHash::get(),
        Reason::InvalidCodeHash
    );
    AllowedNextCodeHash::kill();
    let result = dispatch_call::<T>(code);
    <Module<T>>::deposit_event(Event::AttemptedSetCodeByHash(
        hash,
        result.map(|_| ()).map_err(|e| e.error),
    ));
    Ok(())
}

#[cfg(test)]
fn dispatch_call<T: Config>(_code: Vec<u8>) -> DispatchResultWithPostInfo {
    Ok(frame_support::weights::PostDispatchInfo {
        actual_weight: None,
        pays_fee: frame_support::weights::Pays::No,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;
    use frame_support::storage::StorageValue;

    #[test]
    fn test_allow_next_code_with_hash() {
        new_test_ext().execute_with(|| {
            assert_eq!(allow_next_code_with_hash::<Test>([1u8; 32]), Ok(()));
            assert_eq!(AllowedNextCodeHash::get(), Some([1u8; 32]));
        });
    }

    #[test]
    fn test_allow_next_code_with_hash_event() {
        new_test_ext().execute_with(|| {
            let events_pre: Vec<_> = System::events().into_iter().collect();

            assert_eq!(allow_next_code_with_hash::<Test>([1u8; 32]), Ok(()));

            let events_post: Vec<_> = System::events().into_iter().collect();
            assert_eq!(events_pre.len() + 1, events_post.len());

            let allowed_next_code_hash_event = events_post.into_iter().next().unwrap();

            assert_eq!(
                mock::Event::pallet_cash(crate::Event::AllowedNextCodeHash([1u8; 32])),
                allowed_next_code_hash_event.event
            );
        });
    }

    #[test]
    fn test_set_next_code_via_hash_mismatch() {
        new_test_ext().execute_with(|| {
            let events_pre: Vec<_> = System::events().into_iter().collect();
            AllowedNextCodeHash::put([1u8; 32]);

            assert_eq!(
                set_next_code_via_hash::<Test>(vec![1, 2, 3]),
                Err(Reason::InvalidCodeHash)
            );

            let events_post: Vec<_> = System::events().into_iter().collect();
            assert_eq!(events_pre.len(), events_post.len());
            assert_eq!(AllowedNextCodeHash::get(), Some([1u8; 32]));
        });
    }

    #[test]
    fn test_set_next_code_via_hash() {
        new_test_ext().execute_with(|| {
            let new_code = vec![1, 2, 3];
            let hash = <Gateway as Chain>::hash_bytes(&new_code);
            let events_pre: Vec<_> = System::events().into_iter().collect();
            AllowedNextCodeHash::put(hash);

            assert_eq!(set_next_code_via_hash::<Test>(new_code), Ok(()));

            let events_post: Vec<_> = System::events().into_iter().collect();
            assert_eq!(events_pre.len() + 1, events_post.len());
            assert_eq!(AllowedNextCodeHash::get(), None);

            // Check emitted `AttemptedSetCodeByHash` event
            let attempted_set_code_event = events_post.into_iter().last().unwrap();
            assert_eq!(
                mock::Event::pallet_cash(crate::Event::AttemptedSetCodeByHash(hash, Ok(()))),
                attempted_set_code_event.event
            );
        });
    }
}
