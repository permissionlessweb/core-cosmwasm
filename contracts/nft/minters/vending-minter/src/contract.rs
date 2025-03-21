use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, MintCountResponse, MintPriceResponse, MintableNumTokensResponse,
    QueryMsg, StartTimeResponse,
};
use crate::state::{
    Config, ConfigExtension, CONFIG, CW721_ADDRESS, MINTABLE_NUM_TOKENS, MINTABLE_TOKEN_POSITIONS,
    MINTER_ADDRS, STATUS,
};
use crate::validation::{check_dynamic_per_address_limit, get_three_percent_of_tokens};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, ensure, instantiate2_address, to_json_binary, Addr, BankMsg, Binary, CanonicalAddr, Coin,
    CosmosMsg, Decimal, Deps, DepsMut, Empty, Env, Event, Fraction, MessageInfo, Order, Reply,
    ReplyOn, Response, StdError, StdResult, SubMsg, Timestamp, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw721::msg::CollectionExtensionMsg;
use cw721_base::msg::{ExecuteMsg as cw721ExecuteMsg, InstantiateMsg as Cw721InstantiateMsg};
use cw_utils::{may_pay, maybe_addr, nonpayable};
use earlybird::msg::{
    ConfigResponse as EarlybirdConfigResponse, HasMemberResponse, QueryMsg as EarlybirdQueryMsg,
};
use factory_utils::query::FactoryUtilsQueryMsg;
use minter_utils::{MinterConfig, Status, StatusResponse, SudoMsg};
use rand_core::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro128PlusPlus;
use semver::Version;
use sha2::{Digest, Sha256};
use shuffle::{fy::FisherYates, shuffler::Shuffler};
use std::convert::TryInto;
use terp_fee::{checked_fair_burn, ibc_denom_fair_burn};
use terp_sdk::{generate_instantiate_salt, GENESIS_MINT_START_TIME, NATIVE_DENOM};
use url::Url;
use vending_factory::msg::{ParamsResponse, VendingMinterCreateMsg};
use vending_factory::state::VendingMinterParams;
pub struct TokenPositionMapping {
    pub position: u32,
    pub token_id: u32,
}

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:terp-minter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_CW721_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: VendingMinterCreateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let factory = info.sender.clone();

    // Make sure the sender is the factory contract
    // This will fail if the sender cannot parse a response from the factory contract
    let factory_response: ParamsResponse = deps
        .querier
        .query_wasm_smart(factory.clone(), &FactoryUtilsQueryMsg::Params {})?;
    let factory_params = factory_response.params;

    // set default status so it can be queried without failing
    STATUS.save(deps.storage, &Status::default())?;

    if !check_dynamic_per_address_limit(
        msg.init_msg.per_address_limit,
        msg.init_msg.num_tokens,
        factory_params.extension.max_per_address_limit,
    )? {
        return Err(ContractError::InvalidPerAddressLimit {
            max: display_max_mintable_tokens(
                msg.init_msg.per_address_limit,
                msg.init_msg.num_tokens,
                factory_params.extension.max_per_address_limit,
            )?,
            min: 1,
            got: msg.init_msg.per_address_limit,
        });
    }

    // sanitize base token uri
    let mut base_token_uri = msg.init_msg.base_token_uri.trim().to_string();
    // Check that base_token_uri is a valid IPFS uri
    let parsed_token_uri = Url::parse(&base_token_uri)?;
    if parsed_token_uri.scheme() != "ipfs" {
        return Err(ContractError::InvalidBaseTokenURI {});
    }
    base_token_uri = parsed_token_uri.to_string();

    let genesis_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    // If start time is before genesis time return error
    if msg.init_msg.start_time < genesis_time {
        return Err(ContractError::BeforeGenesisTime {});
    }

    // If current time is beyond the provided start time return error
    if env.block.time > msg.init_msg.start_time {
        return Err(ContractError::InvalidStartTime(
            msg.init_msg.start_time,
            env.block.time,
        ));
    }

    // Validate address for the optional earlybird contract
    let earlybird_addr = msg
        .init_msg
        .earlybird
        .and_then(|w| deps.api.addr_validate(w.as_str()).ok());

    if let Some(eb) = earlybird_addr.clone() {
        // check the earlybird exists
        let res: EarlybirdConfigResponse = deps
            .querier
            .query_wasm_smart(eb, &EarlybirdQueryMsg::Config {})?;
        if res.is_active {
            return Err(ContractError::EarlybirdAlreadyStarted {});
        }
    }

    // Use default start trading time if not provided
    let mut collection_info = msg.collection_params.info.clone();
    let offset = factory_params.max_trading_offset_secs;
    let default_start_time_with_offset = msg.init_msg.start_time.plus_seconds(offset);
    if let Some(start_trading_time) = msg.collection_params.info.start_trading_time {
        // If trading start time > start_time + offset, return error
        if start_trading_time > default_start_time_with_offset {
            return Err(ContractError::InvalidStartTradingTime(
                start_trading_time,
                default_start_time_with_offset,
            ));
        }
    }
    let start_trading_time = msg
        .collection_params
        .info
        .start_trading_time
        .or(Some(default_start_time_with_offset));
    collection_info.start_trading_time = start_trading_time;

    let config = Config {
        factory: factory.clone(),
        collection_code_id: msg.collection_params.code_id,
        extension: ConfigExtension {
            admin: info.sender.clone(),
            payment_address: maybe_addr(deps.api, msg.init_msg.payment_address)?,
            base_token_uri,
            num_tokens: msg.init_msg.num_tokens,
            per_address_limit: msg.init_msg.per_address_limit,
            earlybird: earlybird_addr,
            start_time: msg.init_msg.start_time,
            discount_price: None,
        },
        mint_price: msg.init_msg.mint_price,
    };

    CONFIG.save(deps.storage, &config)?;
    MINTABLE_NUM_TOKENS.save(deps.storage, &msg.init_msg.num_tokens)?;

    let token_ids = random_token_list(
        &env,
        info.sender.clone(),
        (1..=msg.init_msg.num_tokens).collect::<Vec<u32>>(),
    )?;
    // Save mintable token ids map
    let mut token_position = 1;
    for token_id in token_ids {
        MINTABLE_TOKEN_POSITIONS.save(deps.storage, token_position, &token_id)?;
        token_position += 1;
    }
    let collection_checksum = Binary::new(
        deps.querier
            .query_wasm_code_info(msg.collection_params.code_id)?
            .checksum
            .into(),
    );

    let salt1 = generate_instantiate_salt(&collection_checksum, env.block.height);

    // predict the infused collection contract address
    let infusion_addr = match instantiate2_address(
        collection_checksum.as_slice(),
        &deps.api.addr_canonicalize(env.contract.address.as_str())?,
        salt1.as_slice(),
    ) {
        Ok(addr) => addr,
        Err(err) => return Err(ContractError::from(err)),
    };
    // Submessage to instantiate cw721 contract
    let submsg = SubMsg {
        msg: WasmMsg::Instantiate2 {
            code_id: msg.collection_params.code_id,
            msg: to_json_binary(&Cw721InstantiateMsg {
                name: msg.collection_params.name.clone(),
                symbol: msg.collection_params.symbol,
                minter: Some(env.contract.address.to_string()),
                creator: Some(info.sender.to_string()),
                withdraw_address: None,
                collection_info_extension: Some(CollectionExtensionMsg {
                    description: msg.collection_params.info.description.clone(),
                    image: collection_info.image.clone(),
                    external_link: collection_info.external_link.clone(),
                    explicit_content: collection_info.explicit_content.clone(),
                    start_trading_time: start_trading_time.clone(),
                    royalty_info: collection_info.royalty_info.clone(),
                }),
            })?,
            funds: info.funds,
            admin: Some(config.extension.admin.to_string()),
            label: format!("cw721-{}", msg.collection_params.name.trim()),
            salt: salt1.clone(),
        }
        .into(),
        id: INSTANTIATE_CW721_REPLY_ID,
        gas_limit: None,
        reply_on: ReplyOn::Success,
        payload: infusion_addr.into(),
    };

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("contract_name", CONTRACT_NAME)
        .add_attribute("contract_version", CONTRACT_VERSION)
        .add_attribute("sender", factory)
        .add_submessage(submsg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Mint {} => execute_mint_sender(deps, env, info),
        ExecuteMsg::Purge {} => execute_purge(deps, env, info),
        ExecuteMsg::UpdateMintPrice { price } => execute_update_mint_price(deps, env, info, price),
        ExecuteMsg::UpdateStartTime(time) => execute_update_start_time(deps, env, info, time),
        ExecuteMsg::UpdateStartTradingTime(time) => {
            execute_update_start_trading_time(deps, env, info, time)
        }
        ExecuteMsg::UpdatePerAddressLimit { per_address_limit } => {
            execute_update_per_address_limit(deps, env, info, per_address_limit)
        }
        ExecuteMsg::MintTo { recipient } => execute_mint_to(deps, env, info, recipient),
        ExecuteMsg::MintFor {
            token_id,
            recipient,
        } => execute_mint_for(deps, env, info, token_id, recipient),
        ExecuteMsg::SetEarlybird { earlybird } => {
            execute_set_earlybird(deps, env, info, &earlybird)
        }
        ExecuteMsg::Shuffle {} => execute_shuffle(deps, env, info),
        ExecuteMsg::BurnRemaining {} => execute_burn_remaining(deps, env, info),
        ExecuteMsg::UpdateDiscountPrice { price } => {
            execute_update_discount_price(deps, env, info, price)
        }
        ExecuteMsg::RemoveDiscountPrice {} => execute_remove_discount_price(deps, info),
    }
}

pub fn execute_update_discount_price(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    price: u128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.extension.admin {
        return Err(ContractError::Unauthorized(
            "Sender is not an admin".to_owned(),
        ));
    }
    if env.block.time < config.extension.start_time {
        return Err(ContractError::BeforeMintStartTime {});
    }

    // discount price can't be greater than unit price
    if price > config.mint_price.amount.u128() {
        return Err(ContractError::UpdatedMintPriceTooHigh {
            allowed: config.mint_price.amount.u128(),
            updated: price,
        });
    }

    let factory: ParamsResponse = deps
        .querier
        .query_wasm_smart(config.clone().factory, &FactoryUtilsQueryMsg::Params {})?;
    let factory_params = factory.params;

    if factory_params.min_mint_price.amount.u128() > price {
        return Err(ContractError::InsufficientMintPrice {
            expected: factory_params.min_mint_price.amount.u128(),
            got: price,
        });
    }

    config.extension.discount_price = Some(coin(price, config.mint_price.denom.clone()));
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_discount_price")
        .add_attribute("sender", info.sender)
        .add_attribute("discount_price", price.to_string()))
}

pub fn execute_remove_discount_price(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.extension.admin {
        return Err(ContractError::Unauthorized(
            "Sender is not an admin".to_owned(),
        ));
    }
    config.extension.discount_price = None;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "remove_discount_price")
        .add_attribute("sender", info.sender))
}

// Purge frees data after a mint is sold out
// Anyone can purge
pub fn execute_purge(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    // check mint sold out
    let mintable_num_tokens = MINTABLE_NUM_TOKENS.load(deps.storage)?;
    if mintable_num_tokens != 0 {
        return Err(ContractError::NotSoldOut {});
    }

    let keys = MINTER_ADDRS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<Vec<_>>();
    for key in keys {
        MINTER_ADDRS.remove(deps.storage, &key?);
    }

    Ok(Response::new()
        .add_attribute("action", "purge")
        .add_attribute("contract", env.contract.address.to_string())
        .add_attribute("sender", info.sender))
}

// Anyone can pay to shuffle at any time
// Introduces another source of randomness to minting
// There's a fee because this action is expensive.
pub fn execute_shuffle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut res = Response::new();

    let config = CONFIG.load(deps.storage)?;

    let factory: ParamsResponse = deps
        .querier
        .query_wasm_smart(config.factory, &FactoryUtilsQueryMsg::Params {})?;
    let factory_params = factory.params;

    // Check exact shuffle fee payment included in message
    checked_fair_burn(
        &info,
        factory_params.extension.shuffle_fee.amount.u128(),
        None,
        &mut res,
    )?;

    // Check not sold out
    let mintable_num_tokens = MINTABLE_NUM_TOKENS.load(deps.storage)?;
    if mintable_num_tokens == 0 {
        return Err(ContractError::SoldOut {});
    }

    // get positions and token_ids, then randomize token_ids and reassign positions
    let mut positions = vec![];
    let mut token_ids = vec![];
    for mapping in MINTABLE_TOKEN_POSITIONS.range(deps.storage, None, None, Order::Ascending) {
        let (position, token_id) = mapping?;
        positions.push(position);
        token_ids.push(token_id);
    }
    let randomized_token_ids = random_token_list(&env, info.sender.clone(), token_ids.clone())?;
    for (i, position) in positions.iter().enumerate() {
        MINTABLE_TOKEN_POSITIONS.save(deps.storage, *position, &randomized_token_ids[i])?;
    }

    Ok(res
        .add_attribute("action", "shuffle")
        .add_attribute("sender", info.sender))
}

pub fn execute_set_earlybird(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    earlybird: &str,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let mut config = CONFIG.load(deps.storage)?;
    let MinterConfig {
        factory,
        extension:
            ConfigExtension {
                earlybird: existing_earlybird,
                admin,
                start_time,
                ..
            },
        ..
    } = config.clone();
    ensure!(
        admin == info.sender,
        ContractError::Unauthorized("Sender is not an admin".to_owned())
    );

    ensure!(
        env.block.time < start_time,
        ContractError::AlreadyStarted {}
    );

    if let Some(eb) = existing_earlybird {
        let res: EarlybirdConfigResponse = deps
            .querier
            .query_wasm_smart(eb, &EarlybirdQueryMsg::Config {})?;

        ensure!(!res.is_active, ContractError::EarlybirdAlreadyStarted {});
    }

    let new_wl = deps.api.addr_validate(earlybird)?;
    config.extension.earlybird = Some(new_wl.clone());
    // check that the new earlybird exists
    let EarlybirdConfigResponse {
        is_active: wl_is_active,
        mint_price: wl_mint_price,
        ..
    } = deps
        .querier
        .query_wasm_smart(new_wl, &EarlybirdQueryMsg::Config {})?;

    ensure!(!wl_is_active, ContractError::EarlybirdAlreadyStarted {});

    ensure!(
        wl_mint_price.denom == config.mint_price.denom,
        ContractError::InvalidDenom {
            expected: config.mint_price.denom,
            got: wl_mint_price.denom,
        }
    );

    // Earlybird could be free, while factory minimum is not
    let ParamsResponse {
        params:
            VendingMinterParams {
                min_mint_price: factory_min_mint_price,
                ..
            },
    } = deps
        .querier
        .query_wasm_smart(factory, &FactoryUtilsQueryMsg::Params {})?;

    ensure!(
        factory_min_mint_price.amount <= wl_mint_price.amount,
        ContractError::InsufficientEarlybirdMintPrice {
            expected: factory_min_mint_price.amount.into(),
            got: wl_mint_price.amount.into(),
        }
    );

    // Earlybird denom should match factory mint denom
    ensure!(
        factory_min_mint_price.denom == wl_mint_price.denom,
        ContractError::InvalidDenom {
            expected: factory_min_mint_price.denom,
            got: wl_mint_price.denom,
        }
    );

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_attribute("action", "set_earlybird")
        .add_attribute("earlybird", earlybird.to_string()))
}

pub fn execute_mint_sender(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let action = "mint_sender";

    // If there is no active earlybird right now, check public mint
    // Check if after start_time
    if is_public_mint(deps.as_ref(), &info)? && (env.block.time < config.extension.start_time) {
        return Err(ContractError::BeforeMintStartTime {});
    }

    // Check if already minted max per address limit
    let mint_count = mint_count(deps.as_ref(), &info)?;
    if mint_count >= config.extension.per_address_limit {
        return Err(ContractError::MaxPerAddressLimitExceeded {});
    }

    _execute_mint(deps, env, info, action, false, None, None)
}

// Check if a earlybird exists and not ended
// Sender has to be earlybirded to mint
fn is_public_mint(deps: Deps, info: &MessageInfo) -> Result<bool, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // If there is no earlybird, there's only a public mint
    if config.extension.earlybird.is_none() {
        return Ok(true);
    }

    let earlybird = config.extension.earlybird.unwrap();

    let wl_config: EarlybirdConfigResponse = deps
        .querier
        .query_wasm_smart(earlybird.clone(), &EarlybirdQueryMsg::Config {})?;

    if !wl_config.is_active {
        return Ok(true);
    }

    let res: HasMemberResponse = deps.querier.query_wasm_smart(
        earlybird,
        &EarlybirdQueryMsg::HasMember {
            member: info.sender.to_string(),
        },
    )?;
    if !res.has_member {
        return Err(ContractError::NotEarlybirded {
            addr: info.sender.to_string(),
        });
    }

    // Check eb per address limit
    let mint_count = mint_count(deps, info)?;
    if mint_count >= wl_config.per_address_limit {
        return Err(ContractError::MaxPerAddressLimitExceeded {});
    }

    Ok(false)
}

pub fn execute_mint_to(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
) -> Result<Response, ContractError> {
    let recipient = deps.api.addr_validate(&recipient)?;
    let config = CONFIG.load(deps.storage)?;
    let action = "mint_to";

    // Check only admin
    if info.sender != config.extension.admin {
        return Err(ContractError::Unauthorized(
            "Sender is not an admin".to_owned(),
        ));
    }

    _execute_mint(deps, env, info, action, true, Some(recipient), None)
}

pub fn execute_mint_for(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: u32,
    recipient: String,
) -> Result<Response, ContractError> {
    let recipient = deps.api.addr_validate(&recipient)?;
    let config = CONFIG.load(deps.storage)?;
    let action = "mint_for";

    // Check only admin
    if info.sender != config.extension.admin {
        return Err(ContractError::Unauthorized(
            "Sender is not an admin".to_owned(),
        ));
    }

    _execute_mint(
        deps,
        env,
        info,
        action,
        true,
        Some(recipient),
        Some(token_id),
    )
}

// Generalize checks and mint message creation
// mint -> _execute_mint(recipient: None, token_id: None)
// mint_to(recipient: "friend") -> _execute_mint(Some(recipient), token_id: None)
// mint_for(recipient: "friend2", token_id: 420) -> _execute_mint(recipient, token_id)
fn _execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: &str,
    is_admin: bool,
    recipient: Option<Addr>,
    token_id: Option<u32>,
) -> Result<Response, ContractError> {
    let mintable_num_tokens = MINTABLE_NUM_TOKENS.load(deps.storage)?;
    if mintable_num_tokens == 0 {
        return Err(ContractError::SoldOut {});
    }

    let config = CONFIG.load(deps.storage)?;

    if let Some(token_id) = token_id {
        if token_id == 0 || token_id > config.extension.num_tokens {
            return Err(ContractError::InvalidTokenId {});
        }
    }

    let cw721_address = CW721_ADDRESS.load(deps.storage)?;

    let recipient_addr = match recipient {
        Some(some_recipient) => some_recipient,
        None => info.sender.clone(),
    };

    let mint_price: Coin = mint_price(deps.as_ref(), is_admin)?;
    // Exact payment only accepted
    let payment = may_pay(&info, &mint_price.denom)?;
    if payment != mint_price.amount {
        return Err(ContractError::IncorrectPaymentAmount(
            coin(payment.u128(), &config.mint_price.denom),
            mint_price,
        ));
    }

    let mut res = Response::new();

    let factory: ParamsResponse = deps
        .querier
        .query_wasm_smart(config.factory, &FactoryUtilsQueryMsg::Params {})?;
    let factory_params = factory.params;

    // Create network fee msgs
    let mint_fee = if is_admin {
        Decimal::bps(factory_params.extension.airdrop_mint_fee_bps)
    } else {
        Decimal::bps(factory_params.mint_fee_bps)
    };

    let network_fee = mint_price
        .amount
        .checked_multiply_ratio(mint_fee.numerator(), mint_fee.denominator())?;

    // send non-native fees to community pool
    if mint_price.denom != NATIVE_DENOM {
        // only send non-zero amounts
        if !network_fee.is_zero() {
            ibc_denom_fair_burn(
                coin(network_fee.u128(), mint_price.clone().denom),
                None,
                &mut res,
            )?;
        }
    } else {
        checked_fair_burn(&info, network_fee.u128(), None, &mut res)?;
    }

    let mintable_token_mapping = match token_id {
        Some(token_id) => {
            // set position to invalid value, iterate to find matching token_id
            // if token_id not found, token_id is already sold, position is unchanged and throw err
            // otherwise return position and token_id
            let mut position = 0;
            for res in MINTABLE_TOKEN_POSITIONS.range(deps.storage, None, None, Order::Ascending) {
                let (pos, id) = res?;
                if id == token_id {
                    position = pos;
                    break;
                }
            }
            if position == 0 {
                return Err(ContractError::TokenIdAlreadySold { token_id });
            }
            TokenPositionMapping { position, token_id }
        }
        None => random_mintable_token_mapping(deps.as_ref(), env, info.sender.clone())?,
    };

    // Create mint msgs
    let mint_msg = cw721ExecuteMsg::Mint {
        token_id: mintable_token_mapping.token_id.to_string(),
        owner: recipient_addr.to_string(),
        token_uri: Some(format!(
            "{}/{}",
            config.extension.base_token_uri, mintable_token_mapping.token_id
        )),
        extension: None,
    };
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cw721_address.to_string(),
        msg: to_json_binary(&mint_msg)?,
        funds: vec![],
    });
    res = res.add_message(msg);

    // Remove mintable token position from map
    MINTABLE_TOKEN_POSITIONS.remove(deps.storage, mintable_token_mapping.position);
    let mintable_num_tokens = MINTABLE_NUM_TOKENS.load(deps.storage)?;
    // Decrement mintable num tokens
    MINTABLE_NUM_TOKENS.save(deps.storage, &(mintable_num_tokens - 1))?;
    // Save the new mint count for the sender's address
    let new_mint_count = mint_count(deps.as_ref(), &info)? + 1;
    MINTER_ADDRS.save(deps.storage, &info.sender, &new_mint_count)?;

    let seller_amount = if !is_admin {
        let amount = mint_price.amount - network_fee;
        let payment_address = config.extension.payment_address;
        let seller = config.extension.admin;
        // Sending 0 coins fails, so only send if amount is non-zero
        if !amount.is_zero() {
            let msg = BankMsg::Send {
                to_address: payment_address.unwrap_or(seller).to_string(),
                amount: vec![coin(amount.u128(), mint_price.clone().denom)],
            };
            res = res.add_message(msg);
        }
        amount
    } else {
        Uint128::zero()
    };

    Ok(res
        .add_attribute("action", action)
        .add_attribute("sender", info.sender)
        .add_attribute("recipient", recipient_addr)
        .add_attribute("token_id", mintable_token_mapping.token_id.to_string())
        .add_attribute(
            "network_fee",
            coin(network_fee.u128(), mint_price.clone().denom).to_string(),
        )
        .add_attribute("mint_price", mint_price.to_string())
        .add_attribute(
            "seller_amount",
            coin(seller_amount.u128(), mint_price.denom).to_string(),
        ))
}

fn random_token_list(
    env: &Env,
    sender: Addr,
    mut tokens: Vec<u32>,
) -> Result<Vec<u32>, ContractError> {
    let tx_index = if let Some(tx) = &env.transaction {
        tx.index
    } else {
        0
    };
    let sha256 = Sha256::digest(
        format!("{}{}{}{}", sender, env.block.height, tokens.len(), tx_index).into_bytes(),
    );
    // Cut first 16 bytes from 32 byte value
    let randomness: [u8; 16] = sha256.to_vec()[0..16].try_into().unwrap();
    let mut rng = Xoshiro128PlusPlus::from_seed(randomness);
    let mut shuffler = FisherYates::default();
    shuffler
        .shuffle(&mut tokens, &mut rng)
        .map_err(StdError::generic_err)?;
    Ok(tokens)
}

// Does a baby shuffle, picking a token_id from the first or last 50 mintable positions.
fn random_mintable_token_mapping(
    deps: Deps,
    env: Env,
    sender: Addr,
) -> Result<TokenPositionMapping, ContractError> {
    let num_tokens = MINTABLE_NUM_TOKENS.load(deps.storage)?;
    let tx_index = if let Some(tx) = &env.transaction {
        tx.index
    } else {
        0
    };
    let sha256 = Sha256::digest(
        format!("{}{}{}{}", sender, num_tokens, env.block.height, tx_index).into_bytes(),
    );
    // Cut first 16 bytes from 32 byte value
    let randomness: [u8; 16] = sha256.to_vec()[0..16].try_into().unwrap();

    let mut rng = Xoshiro128PlusPlus::from_seed(randomness);

    let r = rng.next_u32();

    let order = match r % 2 {
        1 => Order::Descending,
        _ => Order::Ascending,
    };
    let mut rem = 50;
    if rem > num_tokens {
        rem = num_tokens;
    }
    let n = r % rem;
    let position = MINTABLE_TOKEN_POSITIONS
        .keys(deps.storage, None, None, order)
        .skip(n as usize)
        .take(1)
        .collect::<StdResult<Vec<_>>>()?[0];

    let token_id = MINTABLE_TOKEN_POSITIONS.load(deps.storage, position)?;
    Ok(TokenPositionMapping { position, token_id })
}

pub fn execute_update_mint_price(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    price: u128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.extension.admin {
        return Err(ContractError::Unauthorized(
            "Sender is not an admin".to_owned(),
        ));
    }
    // If current time is after the stored start time, only allow lowering price
    if env.block.time >= config.extension.start_time && price >= config.mint_price.amount.u128() {
        return Err(ContractError::UpdatedMintPriceTooHigh {
            allowed: config.mint_price.amount.u128(),
            updated: price,
        });
    }

    let factory: ParamsResponse = deps
        .querier
        .query_wasm_smart(config.clone().factory, &FactoryUtilsQueryMsg::Params {})?;
    let factory_params = factory.params;

    if factory_params.min_mint_price.amount.u128() > price {
        return Err(ContractError::InsufficientMintPrice {
            expected: factory_params.min_mint_price.amount.u128(),
            got: price,
        });
    }

    config.mint_price = coin(price, config.mint_price.denom);
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "update_mint_price")
        .add_attribute("sender", info.sender)
        .add_attribute("mint_price", price.to_string()))
}

pub fn execute_update_start_time(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    start_time: Timestamp,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.extension.admin {
        return Err(ContractError::Unauthorized(
            "Sender is not an admin".to_owned(),
        ));
    }
    // If current time is after the stored start time return error
    if env.block.time >= config.extension.start_time {
        return Err(ContractError::AlreadyStarted {});
    }

    // If current time already passed the new start_time return error
    if env.block.time > start_time {
        return Err(ContractError::InvalidStartTime(start_time, env.block.time));
    }

    let genesis_start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    // If the new start_time is before genesis start time return error
    if start_time < genesis_start_time {
        return Err(ContractError::BeforeGenesisTime {});
    }

    config.extension.start_time = start_time;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "update_start_time")
        .add_attribute("sender", info.sender)
        .add_attribute("start_time", start_time.to_string()))
}

pub fn execute_update_start_trading_time(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    start_time: Option<Timestamp>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let config = CONFIG.load(deps.storage)?;
    let _cw721_contract_addr = CW721_ADDRESS.load(deps.storage)?;

    if info.sender != config.extension.admin {
        return Err(ContractError::Unauthorized(
            "Sender is not an admin".to_owned(),
        ));
    }

    // add custom rules here
    let factory_params: ParamsResponse = deps
        .querier
        .query_wasm_smart(config.factory.clone(), &FactoryUtilsQueryMsg::Params {})?;
    let default_start_time_with_offset = config
        .extension
        .start_time
        .plus_seconds(factory_params.params.max_trading_offset_secs);

    if let Some(start_trading_time) = start_time {
        if env.block.time > start_trading_time {
            return Err(ContractError::InvalidStartTradingTime(
                env.block.time,
                start_trading_time,
            ));
        }
        // If new start_trading_time > old start time + offset , return error
        if start_trading_time > default_start_time_with_offset {
            return Err(ContractError::InvalidStartTradingTime(
                start_trading_time,
                default_start_time_with_offset,
            ));
        }
    }

    // // execute cw721 contract
    // let msg = WasmMsg::Execute {
    //     contract_addr: cw721_contract_addr.to_string(),
    //     msg: to_json_binary(&cw721ExecuteMsg::UpdateStartTradingTime(start_time))?,
    //     funds: vec![],
    // };

    Ok(
        Response::new()
            .add_attribute("action", "update_start_trading_time")
            .add_attribute("sender", info.sender), // .add_message(msg)
    )
}

pub fn execute_update_per_address_limit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    per_address_limit: u32,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.extension.admin {
        return Err(ContractError::Unauthorized(
            "Sender is not an admin".to_owned(),
        ));
    }

    let factory: ParamsResponse = deps
        .querier
        .query_wasm_smart(config.factory.clone(), &FactoryUtilsQueryMsg::Params {})?;
    let factory_params = factory.params;

    if per_address_limit == 0 || per_address_limit > factory_params.extension.max_per_address_limit
    {
        return Err(ContractError::InvalidPerAddressLimit {
            max: factory_params.extension.max_per_address_limit,
            min: 1,
            got: per_address_limit,
        });
    }

    if !check_dynamic_per_address_limit(
        per_address_limit,
        config.extension.num_tokens,
        factory_params.extension.max_per_address_limit,
    )? {
        return Err(ContractError::InvalidPerAddressLimit {
            max: display_max_mintable_tokens(
                per_address_limit,
                config.extension.num_tokens,
                factory_params.extension.max_per_address_limit,
            )?,
            min: 1,
            got: per_address_limit,
        });
    }

    config.extension.per_address_limit = per_address_limit;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "update_per_address_limit")
        .add_attribute("sender", info.sender)
        .add_attribute("limit", per_address_limit.to_string()))
}

pub fn execute_burn_remaining(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let config = CONFIG.load(deps.storage)?;
    // Check only admin
    if info.sender != config.extension.admin {
        return Err(ContractError::Unauthorized(
            "Sender is not an admin".to_owned(),
        ));
    }

    // check mint not sold out
    let mintable_num_tokens = MINTABLE_NUM_TOKENS.load(deps.storage)?;
    if mintable_num_tokens == 0 {
        return Err(ContractError::SoldOut {});
    }

    let keys = MINTABLE_TOKEN_POSITIONS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<Vec<_>>();
    let mut total: u32 = 0;
    for key in keys {
        total += 1;
        MINTABLE_TOKEN_POSITIONS.remove(deps.storage, key?);
    }
    // Decrement mintable num tokens
    MINTABLE_NUM_TOKENS.save(deps.storage, &(mintable_num_tokens - total))?;

    let event = Event::new("burn-remaining")
        .add_attribute("sender", info.sender)
        .add_attribute("tokens_burned", total.to_string())
        .add_attribute("minter", env.contract.address.to_string());
    Ok(Response::new().add_event(event))
}

// if admin_no_fee => no fee,
// else if in earlybird => earlybird price
// else => config unit price
pub fn mint_price(deps: Deps, is_admin: bool) -> Result<Coin, StdError> {
    let config = CONFIG.load(deps.storage)?;

    let factory: ParamsResponse = deps
        .querier
        .query_wasm_smart(config.factory, &FactoryUtilsQueryMsg::Params {})?;
    let factory_params = factory.params;

    if is_admin {
        return Ok(coin(
            factory_params.extension.airdrop_mint_price.amount.u128(),
            factory_params.extension.airdrop_mint_price.denom,
        ));
    }

    if config.extension.earlybird.is_none() {
        let price = config.extension.discount_price.unwrap_or(config.mint_price);
        return Ok(price);
    }

    let earlybird = config.extension.earlybird.unwrap();

    let wl_config: EarlybirdConfigResponse = deps
        .querier
        .query_wasm_smart(earlybird, &EarlybirdQueryMsg::Config {})?;

    if wl_config.is_active {
        Ok(wl_config.mint_price)
    } else {
        let price = config.extension.discount_price.unwrap_or(config.mint_price);
        Ok(price)
    }
}

fn mint_count(deps: Deps, info: &MessageInfo) -> Result<u32, StdError> {
    let mint_count = (MINTER_ADDRS.key(&info.sender).may_load(deps.storage)?).unwrap_or(0);
    Ok(mint_count)
}

pub fn display_max_mintable_tokens(
    per_address_limit: u32,
    num_tokens: u32,
    max_per_address_limit: u32,
) -> Result<u32, ContractError> {
    if per_address_limit > max_per_address_limit {
        return Ok(max_per_address_limit);
    }
    if num_tokens < 100 {
        return Ok(3_u32);
    }
    let three_percent = get_three_percent_of_tokens(num_tokens)?.u128();
    Ok(three_percent as u32)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, _env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::UpdateStatus {
            is_verified,
            is_blocked,
            is_explicit,
        } => update_status(deps, is_verified, is_blocked, is_explicit)
            .map_err(|_| ContractError::UpdateStatus {}),
    }
}

/// Only governance can update contract params
pub fn update_status(
    deps: DepsMut,
    is_verified: bool,
    is_blocked: bool,
    is_explicit: bool,
) -> StdResult<Response> {
    let mut status = STATUS.load(deps.storage)?;
    status.is_verified = is_verified;
    status.is_blocked = is_blocked;
    status.is_explicit = is_explicit;
    STATUS.save(deps.storage, &status)?;

    Ok(Response::new().add_attribute("action", "sudo_update_status"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Status {} => to_json_binary(&query_status(deps)?),
        QueryMsg::StartTime {} => to_json_binary(&query_start_time(deps)?),
        QueryMsg::MintableNumTokens {} => to_json_binary(&query_mintable_num_tokens(deps)?),
        QueryMsg::MintPrice {} => to_json_binary(&query_mint_price(deps)?),
        QueryMsg::MintCount { address } => to_json_binary(&query_mint_count(deps, address)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let cw721_address = CW721_ADDRESS.load(deps.storage)?;

    Ok(ConfigResponse {
        admin: config.extension.admin.to_string(),
        base_token_uri: config.extension.base_token_uri,
        cw721_address: cw721_address.to_string(),
        cw721_code_id: config.collection_code_id,
        num_tokens: config.extension.num_tokens,
        start_time: config.extension.start_time,
        mint_price: config.mint_price,
        per_address_limit: config.extension.per_address_limit,
        earlybird: config.extension.earlybird.map(|w| w.to_string()),
        factory: config.factory.to_string(),
        discount_price: config.extension.discount_price,
    })
}

pub fn query_status(deps: Deps) -> StdResult<StatusResponse> {
    let status = STATUS.load(deps.storage)?;

    Ok(StatusResponse { status })
}

fn query_mint_count(deps: Deps, address: String) -> StdResult<MintCountResponse> {
    let addr = deps.api.addr_validate(&address)?;
    let mint_count = (MINTER_ADDRS.key(&addr).may_load(deps.storage)?).unwrap_or(0);
    Ok(MintCountResponse {
        address: addr.to_string(),
        count: mint_count,
    })
}

fn query_start_time(deps: Deps) -> StdResult<StartTimeResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(StartTimeResponse {
        start_time: config.extension.start_time.to_string(),
    })
}

fn query_mintable_num_tokens(deps: Deps) -> StdResult<MintableNumTokensResponse> {
    let count = MINTABLE_NUM_TOKENS.load(deps.storage)?;
    Ok(MintableNumTokensResponse { count })
}

fn query_mint_price(deps: Deps) -> StdResult<MintPriceResponse> {
    let config = CONFIG.load(deps.storage)?;

    let factory: ParamsResponse = deps
        .querier
        .query_wasm_smart(config.factory, &FactoryUtilsQueryMsg::Params {})?;

    let factory_params = factory.params;

    let current_price = mint_price(deps, false)?;
    let public_price = config.mint_price.clone();
    let earlybird_price: Option<Coin> = if let Some(earlybird) = config.extension.earlybird {
        let wl_config: EarlybirdConfigResponse = deps
            .querier
            .query_wasm_smart(earlybird, &EarlybirdQueryMsg::Config {})?;
        Some(wl_config.mint_price)
    } else {
        None
    };
    let airdrop_price = coin(
        factory_params.extension.airdrop_mint_price.amount.u128(),
        config.mint_price.denom,
    );
    let discount_price = config.extension.discount_price;
    Ok(MintPriceResponse {
        public_price,
        airdrop_price,
        earlybird_price,
        current_price,
        discount_price,
    })
}

// Reply callback triggered from cw721 contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    if reply.id != INSTANTIATE_CW721_REPLY_ID {
        return Err(ContractError::InvalidReplyID {});
    }
    let cw721_addr = deps
        .api
        .addr_humanize(&CanonicalAddr::from(reply.payload))?;
    CW721_ADDRESS.save(deps.storage, &cw721_addr)?;
    Ok(Response::new()
        .add_attribute("action", "instantiate_cw721_reply")
        .add_attribute("cw721_addr", cw721_addr))

    // let reply = parse_reply_instantiate_data(msg);
    // match reply {
    //     Ok(res) => {
    //         let cw721_address = res.contract_address;
    //         CW721_ADDRESS.save(deps.storage, &Addr::unchecked(cw721_address.clone()))?;
    //         Ok(Response::default()
    //             .add_attribute("action", "instantiate_cw721_reply")
    //             .add_attribute("cw721_address", cw721_address))
    //     }
    //     Err(_) => Err(ContractError::InstantiateSg721Error {}),
    // }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    let current_version = cw2::get_contract_version(deps.storage)?;
    if current_version.contract != CONTRACT_NAME {
        return Err(StdError::generic_err("Cannot upgrade to a different contract").into());
    }
    let version: Version = current_version
        .version
        .parse()
        .map_err(|_| StdError::generic_err("Invalid contract version"))?;
    let new_version: Version = CONTRACT_VERSION
        .parse()
        .map_err(|_| StdError::generic_err("Invalid contract version"))?;

    if version > new_version {
        return Err(StdError::generic_err("Cannot upgrade to a previous contract version").into());
    }
    // if same version return
    if version == new_version {
        return Ok(Response::new());
    }

    // set new contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let event = Event::new("migrate")
        .add_attribute("from_name", current_version.contract)
        .add_attribute("from_version", current_version.version)
        .add_attribute("to_name", CONTRACT_NAME)
        .add_attribute("to_version", CONTRACT_VERSION);
    Ok(Response::new().add_event(event))
}
