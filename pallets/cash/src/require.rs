pub use our_std::{fmt, result};

#[macro_export]
macro_rules! require {
    ($expr:expr, $reason:expr) => {
        if !$expr {
            return core::result::Result::Err($reason);
        }
    };
}

#[macro_export]
macro_rules! must {
    ($expr:expr, $reason:expr) => {
        if !$expr {
            core::result::Result::Err($reason)
        } else {
            core::result::Result::Ok(())
        }
    };
}

#[macro_export]
macro_rules! require_min_tx_value {
    ($value:expr) => {
        require!($value >= MIN_TX_VALUE, Reason::MinTxValueNotMet);
    };
}
