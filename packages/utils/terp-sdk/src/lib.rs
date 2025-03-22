mod msg;
mod query;
mod route;

pub const NATIVE_DENOM: &str = "uterp";
pub const TEST_BOND_DENOM: &str = "uterpx";
pub const TEST_FEE_DENOM: &str = "uthiolx";
pub const GENESIS_MINT_START_TIME: u64 = 1647032400000000000;

use cosmwasm_std::{coin, coins, Addr, BankMsg, Binary, Coin, CosmosMsg, HexBinary};
// pub use msg::{TerpMsg, TerpMsgWrapper};

// pub type Response = cosmwasm_std::Response;
// pub type SubMsg = cosmwasm_std::SubMsg<TerpMsgWrapper>;
// pub type CosmosMsg = cosmwasm_std::CosmosMsg<TerpMsgWrapper>;

// pub use query::TerpQuery;
// pub use route::TerpRoute;

// This export is added to all contracts that import this package, signifying that they require
// "terpnet" support on the chain they run on.

// #[no_mangle]
// extern "C" fn requires_terpnetwork() {}

pub fn terps(amount: impl Into<u128>) -> Vec<Coin> {
    coins(amount.into(), NATIVE_DENOM)
}
pub fn thiols(amount: impl Into<u128>) -> Vec<Coin> {
    coins(amount.into(), NATIVE_DENOM)
}

pub fn test_terps(amount: impl Into<u128>) -> Vec<Coin> {
    coins(amount.into(), TEST_BOND_DENOM)
}
pub fn test_thiols(amount: impl Into<u128>) -> Vec<Coin> {
    coins(amount.into(), TEST_FEE_DENOM)
}

pub fn terp(amount: impl Into<u128>) -> Coin {
    coin(amount.into(), NATIVE_DENOM)
}

pub fn thiol(amount: impl Into<u128>) -> Coin {
    coin(amount.into(), NATIVE_DENOM)
}

pub fn test_terp(amount: impl Into<u128>) -> Coin {
    coin(amount.into(), NATIVE_DENOM)
}

pub fn test_thiol(amount: impl Into<u128>) -> Coin {
    coin(amount.into(), NATIVE_DENOM)
}

pub fn send_terps_msg(to_address: &Addr, amount: impl Into<u128>) -> BankMsg {
    BankMsg::Send {
        to_address: to_address.to_string(),
        amount: terps(amount),
    }
}

pub fn send_thiols_msg(to_address: &Addr, amount: impl Into<u128>) -> BankMsg {
    BankMsg::Send {
        to_address: to_address.to_string(),
        amount: thiols(amount),
    }
}

pub fn create_fund_community_pool_msg(amount: Vec<Coin>) -> CosmosMsg {
    CosmosMsg::Bank(cosmwasm_std::BankMsg::Burn { amount }).into()
}



/// Generates the value used with instantiate2, via a hash of the infusers checksum.
pub const SALT_POSTFIX: &[u8] = b"terpyterp";
pub fn generate_instantiate_salt(checksum: &Binary, height: u64) -> Binary {
    let mut hash = Vec::new();
    hash.extend_from_slice(checksum.as_slice());
    hash.extend_from_slice(&height.to_be_bytes());
    let checksum_hash = <sha2::Sha256 as sha2::Digest>::digest(hash);
    let mut result = checksum_hash.to_vec();
    result.extend_from_slice(SALT_POSTFIX);
    Binary::new(result.to_vec())
}
