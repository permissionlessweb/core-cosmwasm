use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("InvalidConfig: {0}")]
    InvalidConfig(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("InvalidCollectionResidual: {0}")]
    InvalidCollectionResidual(String),

    #[error("ResidualNotFound: {0}")]
    ResidualNotFound(String),
}
