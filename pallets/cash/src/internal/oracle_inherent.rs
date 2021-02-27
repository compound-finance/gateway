use crate::{chains::ChainAccount, Call, Config, Module};
#[cfg(feature = "std")]
use codec::Decode;
use codec::Encode;
#[cfg(feature = "std")]
use sp_inherents::ProvideInherentData;
use sp_inherents::{InherentData, InherentIdentifier, IsFatalError, ProvideInherent};
use sp_runtime::RuntimeString;

/// The identifier for the miner inherent.
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"oracle00";

/// TODO!!

/// The type of the inherent.
pub type InherentType = (Vec<(Vec<u8>, Vec<u8>)>, u64);

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

/// Auxiliary trait to extract miner inherent data.
pub trait OracleInherentData {
    /// Get oracle inherent data.
    fn oracle_inherent_data(&self) -> Result<InherentType, sp_inherents::Error>;
}

impl OracleInherentData for InherentData {
    fn oracle_inherent_data(&self) -> Result<InherentType, sp_inherents::Error> {
        self.get_data(&INHERENT_IDENTIFIER) // TODO: Do we need to convert or anything?
            .and_then(|r| r.ok_or_else(|| "Oracle inherent data not found".into()))
    }
}

/// Provide duration since unix epoch in millisecond for timestamp inherent.
#[cfg(feature = "std")]
pub struct InherentDataProvider;

#[cfg(feature = "std")]
impl ProvideInherentData for InherentDataProvider {
    fn inherent_identifier(&self) -> &'static InherentIdentifier {
        &INHERENT_IDENTIFIER
    }

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

    fn error_to_string(&self, error: &[u8]) -> Option<String> {
        InherentError::try_from(&INHERENT_IDENTIFIER, error).map(|e| format!("{:?}", e))
    }
}

fn extract_inherent_data(data: &InherentData) -> Result<InherentType, RuntimeString> {
    data.get_data::<InherentType>(&INHERENT_IDENTIFIER)
        .map_err(|_| RuntimeString::from("Invalid oracle inherent data encoding."))?
        .ok_or_else(|| "Oracle inherent data is not provided.".into())
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
}
