use codec::{Decode, Encode};
use our_std::{Debuggable, RuntimeDebug};

/// Type for reporting failures for reasons outside of our control.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, Debuggable)]
pub enum Reason {
    AssetExtractionNotSupported,
    BadAccount,
    BadAddress,
    BadAsset,
    BadChainId,
    BadSymbol,
    ChainMismatch,
    CryptoError(compound_crypto::CryptoError),
    FailedToSubmitExtrinsic,
    InsufficientChainCash,
    InsufficientLiquidity,
    InsufficientTotalFunds,
    InvalidSignature,
    InvalidSymbol,
    KeyNotFound,
    MathError(MathError),
    MinTxValueNotMet,
    None,
    NoSuchAsset,
    NoticeAlreadySigned,
    NotImplemented,
    ParseIntError,
    RepayTooMuch,
    SignatureMismatch,
}

impl From<MathError> for Reason {
    fn from(err: MathError) -> Self {
        Reason::MathError(err)
    }
}

impl From<compound_crypto::CryptoError> for Reason {
    fn from(err: compound_crypto::CryptoError) -> Self {
        Reason::CryptoError(err)
    }
}

impl our_std::fmt::Display for Reason {
    fn fmt(&self, f: &mut our_std::fmt::Formatter) -> our_std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Type for reporting failures from calculations.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum MathError {
    Overflow,
    Underflow,
    SignMismatch,
    SymbolMismatch,
    PriceNotUSD,
    DivisionByZero,
}
