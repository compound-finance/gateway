use frame_support::storage::{IterableStorageMap, StorageMap};

use crate::{
    internal, reason::Reason, require, types::ValidatorKeys, Config, Event, Module, NextValidators,
    NoticeHolds, SessionInterface,
};

pub fn change_validators<T: Config>(validators: Vec<ValidatorKeys>) -> Result<(), Reason> {
    require!(NoticeHolds::iter().count() == 0, Reason::PendingAuthNotice);

    for validator in validators.iter() {
        require!(
            <T>::SessionInterface::has_next_keys(validator.substrate_id.clone()),
            Reason::ChangeValidatorsError
        );
    }

    for (id, _keys) in NextValidators::iter() {
        NextValidators::take(&id);
    }
    for keys in &validators {
        NextValidators::insert(&keys.substrate_id, keys);
    }

    <Module<T>>::deposit_event(Event::ChangeValidators(validators.clone()));

    internal::notices::dispatch_change_authority_notice::<T>(validators);

    // rotate to the queued session, and rotate from this one when notices are fully signed
    <T>::SessionInterface::rotate_session();

    Ok(())
}
