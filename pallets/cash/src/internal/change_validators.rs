use frame_support::storage::{IterableStorageMap, StorageMap};

use crate::{
    chains::{ChainHash, ChainId},
    core::dispatch_notice_internal,
    notices::{ChangeAuthorityNotice, Notice},
    reason::Reason,
    require,
    types::ValidatorKeys,
    Config, Event, Module, NextValidators, SessionInterface,
};

pub fn change_validators<T: Config>(validators: Vec<ValidatorKeys>) -> Result<(), Reason> {
    for validator in validators.iter() {
        require!(
            <T>::SessionInterface::is_valid_keys(validator.substrate_id.clone()),
            Reason::ChangeValidatorsError
        );
    }

    for (id, _keys) in NextValidators::iter() {
        NextValidators::take(&id);
    }
    for keys in &validators {
        NextValidators::take(&keys.substrate_id);
        NextValidators::insert(&keys.substrate_id, keys);
    }

    <Module<T>>::deposit_event(Event::ChangeValidators(validators.clone()));

    dispatch_notice_internal::<T>(ChainId::Eth, None, true, &|notice_id, parent_hash| {
        Ok(Notice::ChangeAuthorityNotice(match parent_hash {
            ChainHash::Eth(eth_parent_hash) => ChangeAuthorityNotice::Eth {
                id: notice_id,
                parent: eth_parent_hash,
                new_authorities: validators.iter().map(|x| x.eth_address).collect::<Vec<_>>(),
            },
            _ => panic!("not supported"), // TODO: Panic?
        }))
    })?;

    Ok(())
}
