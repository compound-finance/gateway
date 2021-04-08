use crate::chains::ChainId;
use crate::internal::set_yield_next::SetYieldNextError;
use crate::notices::NoticeId;
use crate::rates::RatesError;
use crate::types::Nonce;
use codec::{Decode, Encode};
use gateway_crypto::CryptoError;
use our_std::RuntimeDebug;
use pallet_oracle::error::OracleError;
use trx_request;

use types_derive::Types;

/// Type for reporting failures for reasons outside of our control.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum Reason {
    AssetExtractionNotSupported,
    AssetNotSupported,
    BadAccount,
    BadAddress,
    BadAsset,
    BadChainId,
    BadFactor,
    BadSymbol,
    BadTicker,
    BadUnits,
    ChainMismatch,
    HashMismatch,
    CryptoError(CryptoError),
    FailedToSubmitExtrinsic,
    WorkerFetchError,
    WorkerLocked,
    IncorrectNonce(Nonce, Nonce),
    InKindLiquidation,
    InsufficientChainCash,
    InsufficientLiquidity,
    InsufficientTotalFunds,
    InvalidAPR,
    InvalidCodeHash,
    InvalidLiquidation,
    InvalidUTF8,
    KeyNotFound,
    MathError(MathError),
    MaxForNonCashAsset,
    MinTxValueNotMet,
    None,
    NoPrice,
    NoSuchAsset,
    NoSuchBlock,
    NoSuchQueue,
    NoticeMissing(ChainId, NoticeId),
    NotImplemented,
    OracleError(OracleError),
    RatesError(RatesError),
    RepayTooMuch,
    SelfTransfer,
    SerdeError,
    SetYieldNextError(SetYieldNextError),
    SignatureAccountMismatch,
    SignatureMismatch,
    TimeTravelNotAllowed,
    TrxRequestParseError(TrxReqParseError),
    UnknownValidator,
    InvalidChain,
    PendingAuthNotice,
    ChangeValidatorsError,
}

impl From<Reason> for frame_support::dispatch::DispatchError {
    fn from(reason: Reason) -> frame_support::dispatch::DispatchError {
        // XXX better way to assign codes?
        //  also we can use them to differentiate between inner type variants
        let (index, error, message) = match reason {
            Reason::AssetExtractionNotSupported => {
                (0, 99, "asset extraction not supported XXX temporary")
            }
            Reason::AssetNotSupported => (0, 0, "asset not supported"),
            Reason::BadAccount => (1, 0, "bad account"),
            Reason::BadAddress => (1, 1, "bad address"),
            Reason::BadAsset => (1, 2, "bad asset"),
            Reason::BadChainId => (1, 3, "bad chain id"),
            Reason::BadFactor => (1, 4, "bad factor"),
            Reason::BadSymbol => (1, 5, "bad symbol"),
            Reason::BadTicker => (1, 6, "bad ticker"),
            Reason::BadUnits => (1, 7, "bad units"),
            Reason::ChainMismatch => (2, 0, "chain mismatch"),
            Reason::HashMismatch => (2, 1, "hash mismatch"),
            Reason::CryptoError(_) => (3, 0, "crypto error"),
            Reason::FailedToSubmitExtrinsic => (5, 0, "failed to submit extrinsic"),
            Reason::WorkerFetchError => (6, 0, "worker fetch error"),
            Reason::WorkerLocked => (6, 1, "worker locked"),
            Reason::IncorrectNonce(_, _) => (7, 0, "incorrect nonce"),
            Reason::InKindLiquidation => (8, 0, "in kind liquidation"),
            Reason::InsufficientChainCash => (9, 0, "insufficient chain cash"),
            Reason::InsufficientLiquidity => (9, 1, "insufficient liquidity"),
            Reason::InsufficientTotalFunds => (9, 2, "insufficient total funds"),
            Reason::InvalidAPR => (10, 0, "invalid apr"),
            Reason::InvalidCodeHash => (10, 1, "invalid code hash"),
            Reason::InvalidLiquidation => (10, 2, "invalid liquidation parameters"),
            Reason::InvalidUTF8 => (10, 3, "invalid utf8"),
            Reason::KeyNotFound => (11, 0, "key not found"),
            Reason::MathError(_) => (12, 0, "math error"),
            Reason::MaxForNonCashAsset => (13, 0, "max for non cash asset"),
            Reason::MinTxValueNotMet => (14, 0, "min tx value not met"),
            Reason::None => (15, 0, "none"),
            Reason::NoPrice => (16, 0, "no price"),
            Reason::NoSuchAsset => (16, 1, "no such asset"),
            Reason::NoSuchBlock => (16, 2, "no such block"),
            Reason::NoSuchQueue => (16, 3, "no such queue"),
            Reason::NoticeMissing(_, _) => (17, 0, "notice missing"),
            Reason::NotImplemented => (18, 0, "not implemented"),
            Reason::OracleError(_) => (19, 0, "oracle error"),
            Reason::RatesError(_) => (20, 0, "rates error"),
            Reason::RepayTooMuch => (21, 0, "repay too much"),
            Reason::SelfTransfer => (22, 0, "self transfer"),
            Reason::SerdeError => (23, 0, "serde error"),
            Reason::SetYieldNextError(_) => (24, 0, "set yield next error"),
            Reason::SignatureAccountMismatch => (25, 0, "signature account mismatch"),
            Reason::SignatureMismatch => (25, 1, "signature mismatch"),
            Reason::TimeTravelNotAllowed => (26, 0, "time travel not allowed"),
            Reason::TrxRequestParseError(_) => (27, 0, "trx request parse error"),
            Reason::UnknownValidator => (28, 0, "unknown validator"),
            Reason::InvalidChain => (29, 0, "invalid chain"),
            Reason::PendingAuthNotice => (30, 0, "change auth notice is already pending"),
            Reason::ChangeValidatorsError => (31, 0, "change validators error"),
        };
        frame_support::dispatch::DispatchError::Module {
            index,
            error,
            message: Some(message),
        }
    }
}

impl From<MathError> for Reason {
    fn from(err: MathError) -> Self {
        Reason::MathError(err)
    }
}

impl From<CryptoError> for Reason {
    fn from(err: CryptoError) -> Self {
        Reason::CryptoError(err)
    }
}

impl From<OracleError> for Reason {
    fn from(err: OracleError) -> Self {
        Reason::OracleError(err)
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

impl serde::de::Error for Reason {
    fn custom<T: our_std::fmt::Display>(_msg: T) -> Self {
        Reason::SerdeError
    }
}

impl serde::de::StdError for Reason {}

/// Type for reporting failures from calculations.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum MathError {
    AbnormalFloatingPointResult,
    DivisionByZero,
    Overflow,
    Underflow,
    SignMismatch,
    PriceNotUSD,
    UnitsMismatch,
}

/// Error from parsing trx requests.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
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
