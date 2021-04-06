use codec::{Decode, Encode};
use frame_support;
use gateway_crypto::CryptoError;
use our_std::Debuggable;

use types_derive::Types;

/// Errors coming from the price oracle.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, Debuggable, Types)]
pub enum OracleError {
    BadTicker,
    CryptoError,
    EthAbiParseError,
    HexParseError,
    HttpError,
    InvalidApiEndpoint,
    InvalidKind,
    InvalidReporter,
    InvalidSymbol,
    InvalidTicker,
    InvalidTimestamp,
    JsonParseError,
    NoPriceFeedURL,
    StalePrice,
    SubmitError,
}

impl From<CryptoError> for OracleError {
    fn from(_err: CryptoError) -> Self {
        OracleError::CryptoError
    }
}

impl From<OracleError> for frame_support::dispatch::DispatchError {
    fn from(err: OracleError) -> frame_support::dispatch::DispatchError {
        let (index, error, message) = match err {
            OracleError::BadTicker => (0, 0, "BadTicker"),
            OracleError::CryptoError => (1, 0, "CryptoError"),
            OracleError::EthAbiParseError => (2, 0, "EthAbiParseError"),
            OracleError::HexParseError => (3, 0, "HexParseError"),
            OracleError::HttpError => (4, 0, "HttpError"),
            OracleError::InvalidApiEndpoint => (5, 0, "InvalidApiEndpoint"),
            OracleError::InvalidKind => (6, 0, "InvalidKind"),
            OracleError::InvalidReporter => (7, 0, "InvalidReporter"),
            OracleError::InvalidSymbol => (8, 0, "InvalidSymbol"),
            OracleError::InvalidTicker => (9, 0, "InvalidTicker"),
            OracleError::InvalidTimestamp => (10, 0, "InvalidTimestamp"),
            OracleError::JsonParseError => (11, 0, "JsonParseError"),
            OracleError::NoPriceFeedURL => (12, 0, "NoPriceFeedURL"),
            OracleError::StalePrice => (13, 0, "StalePrice"),
            OracleError::SubmitError => (14, 0, "SubmitError"),
        };
        frame_support::dispatch::DispatchError::Module {
            index,
            error,
            message: Some(message),
        }
    }
}
