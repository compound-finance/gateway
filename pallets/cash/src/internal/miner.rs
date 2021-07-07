use crate::{chains::ChainAccount, Call, Config, Miner, Module};
use codec::{Decode, Encode};
use frame_support::{inherent::ProvideInherent, storage::StorageValue};
use sp_inherents::{InherentData, InherentIdentifier, IsFatalError};
use sp_runtime::RuntimeString;

/// The identifier for the miner inherent.
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"miner000";
/// The type of the inherent.
pub type InherentType = ChainAccount;

/// Errors that can occur while checking the miner inherent.
#[derive(Encode, sp_runtime::RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Decode))]
pub enum InherentError {
    /// Some other error.
    Other(RuntimeString),
}

impl IsFatalError for InherentError {
    fn is_fatal_error(&self) -> bool {
        match self {
            InherentError::Other(_) => true,
        }
    }
}

impl InherentError {
    /// Try to create an instance ouf of the given identifier and data.
    #[cfg(feature = "std")]
    pub fn try_from(id: &InherentIdentifier, data: &[u8]) -> Option<Self> {
        if id == &INHERENT_IDENTIFIER {
            <InherentError as codec::Decode>::decode(&mut &data[..]).ok()
        } else {
            None
        }
    }
}

/// Provide chosen miner address.
#[cfg(feature = "std")]
pub struct InherentDataProvider;

#[cfg(feature = "std")]
impl std::ops::Deref for InherentDataProvider {
    type Target = ();

    fn deref(&self) -> &Self::Target {
        &()
    }
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for InherentDataProvider {
    fn provide_inherent_data(
        &self,
        inherent_data: &mut InherentData,
    ) -> Result<(), sp_inherents::Error> {
        let miner_address_str = runtime_interfaces::validator_config_interface::get_miner_address()
            .ok_or(sp_inherents::Error::Application(Box::from(
                "no miner address",
            )))?;

        let miner_address = our_std::str::from_utf8(&miner_address_str).map_err(|_| {
            sp_inherents::Error::Application(Box::from("invalid miner address bytes"))
        })?;

        let chain_account: ChainAccount = our_std::str::FromStr::from_str(miner_address)
            .map_err(|_| sp_inherents::Error::Application(Box::from("invalid miner address")))?;

        inherent_data.put_data(INHERENT_IDENTIFIER, &chain_account)
    }

    async fn try_handle_error(
        &self,
        identifier: &InherentIdentifier,
        error: &[u8],
    ) -> Option<Result<(), sp_inherents::Error>> {
        if *identifier != INHERENT_IDENTIFIER {
            return None;
        }

        match InherentError::try_from(&INHERENT_IDENTIFIER, error)? {
            e => Some(Err(sp_inherents::Error::Application(Box::from(format!(
                "{:?}",
                e
            ))))),
        }
    }
}

fn extract_inherent_data(data: &InherentData) -> Result<InherentType, RuntimeString> {
    data.get_data::<InherentType>(&INHERENT_IDENTIFIER)
        .map_err(|_| RuntimeString::from("Invalid miner inherent data encoding."))?
        .ok_or_else(|| "Miner inherent data is not provided.".into())
}

impl<T: Config> ProvideInherent for Module<T> {
    type Call = Call<T>;
    type Error = InherentError;
    const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

    fn create_inherent(data: &InherentData) -> Option<Self::Call> {
        match extract_inherent_data(data) {
            Ok(miner) => Some(Call::set_miner(miner)),
            Err(_err) => None,
        }
    }

    fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
        let _t: ChainAccount = match call {
            Call::set_miner(ref t) => t.clone(),
            _ => return Ok(()),
        };

        let _data = extract_inherent_data(data).map_err(|e| InherentError::Other(e))?;

        // We don't actually have any qualms with the miner's choice, so long as it decodes
        Ok(())
    }

    fn is_inherent(call: &Self::Call) -> bool {
        // XXX
        match call {
            Call::set_miner(_) => true,
            _ => false,
        }
    }
}

// Miner might not be set (e.g. in the first block mined), but for accouting
// purposes, we want some address to make sure all numbers tie out. As such,
// let's just give the initial rewards to some burn account.
pub fn get_some_miner<T: Config>() -> ChainAccount {
    Miner::get().unwrap_or(ChainAccount::Eth([0; 20]))
}

pub fn set_miner<T: Config>(miner: ChainAccount) {
    Miner::put(miner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        tests::*,
        *,
    };

    #[test]
    fn test_set_miner_and_get_some_miner() {
        new_test_ext().execute_with(|| {
            assert_eq!(Miner::get(), None);
            assert_eq!(get_some_miner::<Test>(), ChainAccount::Eth([0; 20]));
            set_miner::<Test>(ChainAccount::Eth([1; 20]));
            assert_eq!(Miner::get(), Some(ChainAccount::Eth([1; 20])));
            assert_eq!(get_some_miner::<Test>(), ChainAccount::Eth([1; 20]));
        });
    }
}
