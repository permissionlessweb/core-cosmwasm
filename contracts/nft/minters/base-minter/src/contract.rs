use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg};
use crate::state::{increment_token_index, Config, COLLECTION_ADDRESS, CONFIG, STATUS};
use base_factory::msg::{BaseMinterCreateMsg, ParamsResponse};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    instantiate2_address, to_json_binary, Addr, Binary, CanonicalAddr, CosmosMsg, Decimal, Deps,
    DepsMut, Empty, Env, Fraction, HexBinary, MessageInfo, Reply, Response, StdResult, SubMsg,
    Timestamp, WasmMsg,
};
use cw2::set_contract_version;
use cw721::msg::CollectionExtensionMsg;
use cw721::Ownership;
use cw721_base::msg::{
    ExecuteMsg as cw721ExecuteMsg, InstantiateMsg as Cw721InstantiateMsg, QueryMsg as cw721QueryMsg,
};
use cw_utils::{must_pay, nonpayable};
use factory_utils::query::FactoryUtilsQueryMsg;
use minter_utils::{QueryMsg, Status, StatusResponse, SudoMsg};
use terp_fee::checked_fair_burn;
use terp_sdk::{generate_instantiate_salt, NATIVE_DENOM};
use url::Url;

const CONTRACT_NAME: &str = "crates.io:terp-base-minter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const INSTANTIATE_CW721_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: BaseMinterCreateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let factory = info.sender.clone();

    // set default status so it can be queried without failing
    STATUS.save(deps.storage, &Status::default())?;

    // Make sure the sender is the factory contract
    // This will fail if the sender cannot parse a response from the factory contract
    let factory_params: ParamsResponse = deps
        .querier
        .query_wasm_smart(factory.clone(), &FactoryUtilsQueryMsg::Params {})?;

    let config = Config {
        factory: factory.clone(),
        collection_code_id: msg.collection_params.code_id,
        // assume the mint price is the minimum mint price
        // 100% is fair burned
        mint_price: factory_params.params.min_mint_price,
        extension: Empty {},
    };

    let collection_checksum = deps
        .querier
        .query_wasm_code_info(msg.collection_params.code_id)?
        .checksum;
    let checksum = Binary::new(collection_checksum.into());
    let salt1 = generate_instantiate_salt(&checksum, env.block.height);

    // Use default start trading time if not provided
    let mut collection_info = msg.collection_params.info.clone();
    let offset = factory_params.params.max_trading_offset_secs;
    let start_trading_time = msg
        .collection_params
        .info
        .start_trading_time
        .or_else(|| Some(env.block.time.plus_seconds(offset)));
    collection_info.start_trading_time = start_trading_time;

    let wasm_msg = WasmMsg::Instantiate2 {
        admin: Some(
            deps.api
                .addr_validate(&info.sender.to_string())?
                .to_string(),
        ),
        code_id: msg.collection_params.code_id,
        msg: to_json_binary(&Cw721InstantiateMsg {
            name: msg.collection_params.name.clone(),
            symbol: msg.collection_params.symbol,
            creator: Some(info.sender.to_string()),
            withdraw_address: None,
            minter: Some(env.contract.address.to_string()),
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
        label: format!(
            "cw721-{}-{}",
            msg.collection_params.code_id,
            msg.collection_params.name.trim()
        ),
        salt: salt1.clone(),
    };

    CONFIG.save(deps.storage, &config)?;

    // predict the infused collection contract address
    let infusion_addr = match instantiate2_address(
        collection_checksum.as_slice(),
        &deps.api.addr_canonicalize(env.contract.address.as_str())?,
        salt1.as_slice(),
    ) {
        Ok(addr) => addr,
        Err(err) => return Err(ContractError::from(err)),
    };
    let submsg =
        SubMsg::reply_on_success(wasm_msg, INSTANTIATE_CW721_REPLY_ID).with_payload(infusion_addr);

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
        ExecuteMsg::Mint { token_uri } => execute_mint_sender(deps, info, token_uri),
        ExecuteMsg::UpdateStartTradingTime(time) => {
            execute_update_start_trading_time(deps, env, info, time)
        }
    }
}

pub fn execute_mint_sender(
    deps: DepsMut,
    info: MessageInfo,
    token_uri: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let collection_address = COLLECTION_ADDRESS.load(deps.storage)?;

    // This is a 1:1 minter, minted at min_mint_price
    // Should mint and then list on the marketplace for secondary sales
    let minter_info: Ownership<Addr> = deps.querier.query_wasm_smart(
        collection_address.clone(),
        &cw721QueryMsg::GetMinterOwnership {},
    )?;
    // allow only cw721 creator address to mint
    if let Some(minter) = minter_info.owner {
        if minter != info.sender {
            return Err(ContractError::Unauthorized(
                "Sender is not cw721 creator".to_owned(),
            ));
        };
    }

    let parsed_token_uri = Url::parse(&token_uri)?;
    if parsed_token_uri.scheme() != "ipfs" {
        return Err(ContractError::InvalidTokenURI {});
    }

    let mut res = Response::new();

    let factory: ParamsResponse = deps
        .querier
        .query_wasm_smart(config.factory, &FactoryUtilsQueryMsg::Params {})?;
    let factory_params = factory.params;

    let funds_sent = must_pay(&info, NATIVE_DENOM)?;


    // Create network fee msgs
    let mint_fee_percent = Decimal::bps(factory_params.mint_fee_bps);
    let network_fee = config
        .mint_price
        .amount
        .checked_multiply_ratio(mint_fee_percent.numerator(), mint_fee_percent.denominator())?;
    // For the base 1/1 minter, the entire mint price should be Fair Burned
    if network_fee != funds_sent {
        return Err(ContractError::InvalidMintPrice {});
    }
    checked_fair_burn(&info, network_fee.u128(), None, &mut res)?;

    // Create mint msgs
    let mint_msg = cw721ExecuteMsg::Mint {
        token_id: increment_token_index(deps.storage)?.to_string(),
        owner: info.sender.to_string(),
        token_uri: Some(token_uri.clone()),
        extension: None,
    };
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: collection_address.to_string(),
        msg: to_json_binary(&mint_msg)?,
        funds: vec![],
    });
    res = res.add_message(msg);

    Ok(res
        .add_attribute("action", "mint")
        .add_attribute("sender", info.sender)
        .add_attribute("token_uri", token_uri)
        .add_attribute("network_fee", network_fee.to_string()))
}

pub fn execute_update_start_trading_time(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    start_time: Option<Timestamp>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let cw721_contract_addr = COLLECTION_ADDRESS.load(deps.storage)?;

    let ownership: Ownership<Addr> = deps.querier.query_wasm_smart(
        cw721_contract_addr.clone(),
        &cw721QueryMsg::GetCreatorOwnership {},
    )?;

    if let Some(oh) = ownership.owner {
        if info.sender != oh {
            return Err(ContractError::Unauthorized(
                "Sender is not creator".to_owned(),
            ));
        }
        // add custom rules here
        if let Some(start_time) = start_time {
            if env.block.time > start_time {
                return Err(ContractError::InvalidStartTradingTime(
                    env.block.time,
                    start_time,
                ));
            }
        }
        // execute cw721 contract
        // let msg = WasmMsg::Execute {
        //     contract_addr: cw721_contract_addr.to_string(),
        //     msg: to_json_binary(&cw721ExecuteMsg::UpdateStartTradingTime(start_time))?,
        //     funds: vec![],
        // };
    }

    Ok(
        Response::new()
            .add_attribute("action", "update_start_time")
            .add_attribute("sender", info.sender), // .add_message(msg)
    )
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
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Status {} => to_json_binary(&query_status(deps)?),
    }
}
// Reply callback triggered from cw721 contract instantiation in instantiate()
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    if reply.id != INSTANTIATE_CW721_REPLY_ID {
        return Err(ContractError::InvalidReplyID {});
    }
    let cw721_addr = deps
        .api
        .addr_humanize(&CanonicalAddr::from(reply.payload))?;
    COLLECTION_ADDRESS.save(deps.storage, &cw721_addr)?;
    Ok(Response::new()
        .add_attribute("action", "instantiate_cw721_reply")
        .add_attribute("cw721_addr", cw721_addr))
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

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let collection_address = COLLECTION_ADDRESS.load(deps.storage)?;

    Ok(ConfigResponse {
        collection_address: collection_address.to_string(),
        config,
    })
}

pub fn query_status(deps: Deps) -> StdResult<StatusResponse> {
    let status = STATUS.load(deps.storage)?;

    Ok(StatusResponse { status })
}
