use crate::{oracle, validate_trx, Call, Config, Module};
use codec::{Decode, Encode};
use frame_support::inherent::ProvideInherent;
use our_std::{log, RuntimeDebug};
use sp_inherents::{InherentData, InherentIdentifier, IsFatalError};
use sp_runtime::{transaction_validity::TransactionSource, RuntimeString};

/// The identifier for the miner inherent.
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"oracle00";

/// The type of the inherent.
type Message = Vec<u8>;
type Signature = Vec<u8>;
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct InherentPriceData(Vec<(Message, Signature)>, u64);

/// Errors that can occur while checking the miner inherent.
#[derive(Encode, sp_runtime::RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Decode))]
pub enum InherentError {
    ValidationError(validate_trx::ValidationError),
    /// Some other error.
    Other(RuntimeString),
}

impl IsFatalError for InherentError {
    fn is_fatal_error(&self) -> bool {
        match self {
            InherentError::ValidationError(_) => false,
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

/// Provide open price feed prices.
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
        if let Some(price_feed_data) = runtime_interfaces::price_feed_interface::get_price_data() {
            inherent_data.put_data(INHERENT_IDENTIFIER, &price_feed_data)
        } else {
            Ok(())
        }
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

fn extract_inherent_data(data: &InherentData) -> Result<InherentPriceData, RuntimeString> {
    let data_result = data.get_data::<InherentPriceData>(&INHERENT_IDENTIFIER);

    data_result
        .map_err(|_| RuntimeString::from("Invalid oracle inherent data encoding."))?
        .ok_or_else(|| "Oracle inherent data is not provided.".into())
}

impl<T: Config> ProvideInherent for Module<T> {
    type Call = Call<T>;
    type Error = InherentError;
    const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

    fn create_inherent(data: &InherentData) -> Option<Self::Call> {
        let inherent_data_res = extract_inherent_data(data);

        match inherent_data_res {
            Ok(InherentPriceData(pairs, _ts)) => {
                if pairs
                    .iter()
                    .all(|(payload, _)| oracle::get_and_check_parsed_price::<T>(&payload).is_ok())
                {
                    Some(Call::post_prices(pairs))
                } else {
                    // Our data is stale, let's not post it
                    None
                }
            }
            Err(_err) => None,
        }
    }

    fn check_inherent(call: &Self::Call, _data: &InherentData) -> Result<(), Self::Error> {
        match validate_trx::validate_unsigned(TransactionSource::InBlock, call) {
            Ok(_) => Ok(()),
            Err(err) => {
                log!("Invalid inherent: {:?}", err);
                Err(InherentError::ValidationError(err))
            }
        }
    }

    fn is_inherent(call: &Self::Call) -> bool {
        // XXX
        match call {
            Call::post_prices(_) => true,
            _ => false,
        }
    }
}
