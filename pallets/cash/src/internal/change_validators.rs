use frame_support::storage::{IterableStorageMap, StorageMap};

use crate::{
    internal, reason::Reason, require, types::ValidatorKeys, Config, Event, Module, NextValidators,
    SessionInterface,
};

pub fn change_validators<T: Config>(validators: Vec<ValidatorKeys>) -> Result<(), Reason> {
    for (id, _) in NextValidators::iter() {
        require!(
            <T>::SessionInterface::is_valid_keys(id) == true,
            Reason::ChangeValidatorsError
        );
    }

    for (id, _keys) in <NextValidators>::iter() {
        <NextValidators>::take(&id);
    }
    for keys in &validators {
        <NextValidators>::take(&keys.substrate_id);
        <NextValidators>::insert(&keys.substrate_id, keys);
    }

    <Module<T>>::deposit_event(Event::ChangeValidators(validators.clone()));

    internal::notices::dispatch_change_authority_notice::<T>(validators);

    Ok(())
}
