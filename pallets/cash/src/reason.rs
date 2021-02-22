use crate::chains::ChainId;
use crate::internal::set_yield_next::SetYieldNextError;
use crate::notices::NoticeId;
use crate::rates::RatesError;
use crate::types::Nonce;
use codec::{Decode, Encode};
use our_std::RuntimeDebug;
use trx_request;

/// Type for reporting failures for reasons outside of our control.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum Reason {
    AssetExtractionNotSupported, // XXX temporary?
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
    CryptoError(compound_crypto::CryptoError),
    EventAlreadySigned,
    FailedToSubmitExtrinsic,
    FetchError,
    IncorrectNonce(Nonce, Nonce),
    InKindLiquidation,
    InsufficientChainCash,
    InsufficientLiquidity,
    InsufficientTotalFunds,
    InvalidAPR,
    InvalidCodeHash,
    InvalidUTF8,
    KeyNotFound,
    MathError(MathError),
    MaxForNonCashAsset,
    MinTxValueNotMet,
    None,
    NoPrice,
    NoSuchAsset,
    NoticeAlreadySigned,
    NoticeHashMismatch,
    NoticeMissing(ChainId, NoticeId),
    NotImplemented,
    OracleError(OracleError),
    RatesError(RatesError),
    RepayTooMuch,
    SelfTransfer,
    SignatureAccountMismatch,
    SignatureMismatch,
    TimeTravelNotAllowed,
    TrxRequestParseError(TrxReqParseError),
    UnknownValidator,
    SetYieldNextError(SetYieldNextError),
}

impl From<Reason> for frame_support::dispatch::DispatchError {
    fn from(reason: Reason) -> frame_support::dispatch::DispatchError {
        // XXX better way to assign codes?
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
            Reason::CryptoError(_) => (3, 0, "crypto error"),
            Reason::EventAlreadySigned => (4, 0, "event already signed"),
            Reason::FailedToSubmitExtrinsic => (5, 0, "failed to submit extrinsic"),
            Reason::FetchError => (6, 0, "fetch error"),
            Reason::IncorrectNonce(_, _) => (6, 0, "incorrect nonce"),
            Reason::InKindLiquidation => (6, 0, "in kind liquidation"),
            Reason::InsufficientChainCash => (6, 0, "insufficient chain cash"),
            Reason::InsufficientLiquidity => (6, 1, "insufficient liquidity"),
            Reason::InsufficientTotalFunds => (6, 2, "insufficient total funds"),
            Reason::InvalidAPR => (7, 0, "invalid apr"),
            Reason::InvalidCodeHash => (7, 1, "invalid code hash"),
            Reason::InvalidUTF8 => (7, 2, "invalid utf8"),
            Reason::KeyNotFound => (8, 0, "key not found"),
            Reason::MathError(_) => (9, 0, "math error"),
            Reason::MaxForNonCashAsset => (10, 0, "max for non cash asset"),
            Reason::MinTxValueNotMet => (11, 0, "min tx value not met"),
            Reason::None => (12, 0, "none"),
            Reason::NoPrice => (13, 0, "no price"),
            Reason::NoSuchAsset => (13, 1, "no such asset"),
            Reason::NoticeAlreadySigned => (14, 0, "notice already signed"),
            Reason::NoticeHashMismatch => (14, 1, "notice hash mismatch"),
            Reason::NoticeMissing(_, _) => (14, 2, "notice missing"),
            Reason::NotImplemented => (15, 0, "not implemented"),
            Reason::OracleError(_) => (16, 0, "oracle error"),
            Reason::RatesError(_) => (17, 0, "rates error"),
            Reason::RepayTooMuch => (18, 0, "repay too much"),
            Reason::SelfTransfer => (19, 0, "self transfer"),
            Reason::SignatureAccountMismatch => (20, 0, "signature account mismatch"),
            Reason::SignatureMismatch => (20, 1, "signature mismatch"),
            Reason::TimeTravelNotAllowed => (21, 0, "time travel not allowed"),
            Reason::TrxRequestParseError(_) => (22, 0, "trx request parse error"),
            Reason::UnknownValidator => (23, 0, "unknown validator"),
            Reason::SetYieldNextError(_) => (24, 0, "set yield next error"),
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

impl From<compound_crypto::CryptoError> for Reason {
    fn from(err: compound_crypto::CryptoError) -> Self {
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

/// Errors coming from the price oracle.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum OracleError {
    EthAbiParseError,
    InvalidApiEndpoint,
    InvalidApiResponse,
    InvalidKind,
    InvalidSymbol,
    InvalidTicker,
    JsonParseError,
    HexParseError,
    HttpError,
    NotAReporter,
    StalePrice,
    SubmitError,
}

/// Error from parsing trx requests.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
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
