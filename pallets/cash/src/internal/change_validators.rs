use frame_support::storage::{IterableStorageMap, StorageMap};

use crate::{
    chains::{ChainHash, ChainId},
    core::dispatch_notice_internal,
    notices::{ChangeAuthorityNotice, Notice},
    reason::Reason,
    require,
    types::ValidatorKeys,
    Config, NextValidators, SessionInterface,
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

// TODO:
// #[cfg(test)]
// mod tests {
// }
