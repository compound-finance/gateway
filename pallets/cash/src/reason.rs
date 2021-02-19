use crate::chains::ChainId;
use crate::internal::set_yield_next::SetYieldNextError;
use crate::notices::NoticeId;
use crate::oracle::OracleError;
use crate::rates::RatesError;
use crate::types::Nonce;
use codec::{Decode, Encode};
use our_std::{Debuggable, RuntimeDebug};
use trx_request;

/// Type for reporting failures for reasons outside of our control.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, Debuggable)]
pub enum Reason {
    AssetExtractionNotSupported, // XXX temporary?
    AssetNotSupported,
    BadAccount,
    BadAddress,
    BadAsset,
    BadChainId,
    BadSymbol,
    BadTicker,
    BadUnits,
    ChainMismatch,
    CryptoError(compound_crypto::CryptoError),
    FailedToSubmitExtrinsic,
    HttpError,
    IncorrectNonce(Nonce, Nonce),
    InKindLiquidation,
    InsufficientChainCash,
    InsufficientLiquidity,
    InsufficientTotalFunds,
    InvalidLiquidityFactor,
    InvalidSignature,
    InvalidUTF8,
    KeyNotFound,
    MathError(MathError),
    MinTxValueNotMet,
    MissingEvent,
    NegativePrincipalExtraction,
    NegativePrincipalExtrction,
    None,
    NoSuchAsset,
    NoticeAlreadySigned,
    NoticeMissing(ChainId, NoticeId),
    NotImplemented,
    OracleError(OracleError),
    ParseIntError,
    RatesError(RatesError),
    RepayTooMuch,
    SelfTransfer,
    SignatureAccountMismatch,
    SignatureMismatch,
    StringError,
    TimeTravelNotAllowed,
    TrxRequestParseError(TrxReqParseError),
    UnknownValidator,
    ValidatorAlreadySigned,
    SetYieldNextError(SetYieldNextError),
    MaxForNonCashAsset,
    MaxWithNegativeCashPrincipal,
    MismatchedNoticeHash,
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

impl From<RatesError> for Reason {
    fn from(err: RatesError) -> Self {
        Reason::RatesError(err)
    }
}

impl From<TrxReqParseError> for Reason {
    fn from(err: TrxReqParseError) -> Self {
        Reason::TrxRequestParseError(err)
    }
}

impl From<trx_request::ParseError<'_>> for Reason {
    fn from(err: trx_request::ParseError<'_>) -> Self {
        Reason::TrxRequestParseError(err.into())
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
    AbnormalFloatingPointResult,
    DivisionByZero,
    Overflow,
    Underflow,
    SignMismatch,
    PriceNotUSD,
    UnitsMismatch,
}

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, Debuggable)]
pub enum TrxReqParseError {
    NotImplemented,
    LexError,
    InvalidAmount,
    InvalidAddress,
    InvalidArgs,
    UnknownFunction,
    InvalidExpression,
    InvalidChain,
    InvalidChainAccount,
}

impl From<trx_request::ParseError<'_>> for TrxReqParseError {
    fn from(err: trx_request::ParseError) -> Self {
        match err {
            trx_request::ParseError::NotImplemented => TrxReqParseError::NotImplemented,
            trx_request::ParseError::LexError(_) => TrxReqParseError::LexError,
            trx_request::ParseError::InvalidAmount => TrxReqParseError::InvalidAmount,
            trx_request::ParseError::InvalidAddress => TrxReqParseError::InvalidAddress,
            trx_request::ParseError::InvalidArgs(_, _, _) => TrxReqParseError::InvalidArgs,
            trx_request::ParseError::UnknownFunction(_) => TrxReqParseError::UnknownFunction,
            trx_request::ParseError::InvalidExpression => TrxReqParseError::InvalidExpression,
            trx_request::ParseError::InvalidChain(_) => TrxReqParseError::InvalidChain,
            trx_request::ParseError::InvalidChainAccount(_) => {
                TrxReqParseError::InvalidChainAccount
            }
        }
    }
}
