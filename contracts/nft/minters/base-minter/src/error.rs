use cosmwasm_std::{CheckedMultiplyRatioError, Instantiate2AddressError, StdError, Timestamp};
use cw_utils::PaymentError;
use terp_fee::FeeError;
use thiserror::Error;
use url::ParseError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("{0}")]
    ParseError(#[from] ParseError),

    #[error("{0}")]
    Instantiate2AddressError(#[from] Instantiate2AddressError),

    #[error("{0}")]
    CheckedMultiplyRatioError(#[from] CheckedMultiplyRatioError),

    #[error("{0}")]
    Fee(#[from] FeeError),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("UpdateStatus")]
    UpdateStatus {},

    #[error("InvalidMintPrice")]
    InvalidMintPrice {},

    #[error("InvalidTokenURI")]
    InvalidTokenURI {},

    #[error("Invalid reply ID")]
    InvalidReplyID {},

    #[error("InvalidDenom {expected} got {got}")]
    InvalidDenom { expected: String, got: String },

    #[error("Instantiate cw721 error")]
    InstantiateSg721Error {},

    #[error("InvalidStartTradingTime {0} < {1}")]
    InvalidStartTradingTime(Timestamp, Timestamp),
}
