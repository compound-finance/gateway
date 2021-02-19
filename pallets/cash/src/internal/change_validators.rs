use frame_support::storage::{IterableStorageMap, StorageMap};

use crate::{reason::Reason, types::ValidatorKeys, Config, NextValidators};

pub fn change_validators<T: Config>(validators: Vec<ValidatorKeys>) -> Result<(), Reason> {
    for (id, _keys) in <NextValidators>::iter() {
        <NextValidators>::take(&id);
    }
    for keys in &validators {
        <NextValidators>::take(&keys.substrate_id);
        <NextValidators>::insert(&keys.substrate_id, keys);
    }
    Ok(())
}
