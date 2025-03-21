use crate::common_setup::contract_boxes::contract_cw721_base;
use crate::common_setup::setup_accounts_and_block::{CREATION_FEE, INITIAL_BALANCE};
use crate::common_setup::setup_minter::base_minter::mock_params::mock_params;
use crate::common_setup::setup_minter::common::constants::MIN_MINT_PRICE;
use crate::common_setup::templates::{
    base_minter_with_cw721, base_minter_with_cw721nt, base_minter_with_specified_cw721,
};
use base_factory::msg::{BaseMinterCreateMsg, BaseUpdateParamsMsg, SudoMsg};

use base_minter::msg::{ConfigResponse, ExecuteMsg};
use cosmwasm_std::{coin, coins, Addr, Empty, Timestamp};
use cw721::msg::{Cw721ExecuteMsg, Cw721QueryMsg, OwnerOfResponse};
use cw721_base::msg::QueryMsg as cw721QueryMsg;
use cw_multi_test::Executor;
use factory_utils::msg::FactoryUtilsExecuteMsg;
use factory_utils::query::{AllowedCollectionCodeIdsResponse, FactoryUtilsQueryMsg};
use factory_utils::tests::mock_collection_params_1;
use minter_utils::QueryMsg;
use terp_sdk::{GENESIS_MINT_START_TIME, NATIVE_DENOM};

#[test]
fn test_init() {
    let bmt = base_minter_with_cw721nt(1);
    bmt.collection_response_vec[0].minter.clone().unwrap();
    bmt.collection_response_vec[0].collection.clone().unwrap();
}

#[test]
fn test_update_code_id() {
    let cw721_code_id = 7u64;
    let bmt = base_minter_with_specified_cw721(1, cw721_code_id);
    let (mut router, creator, _) = (bmt.router, bmt.accts.creator, bmt.accts.buyer);
    let factory = bmt.collection_response_vec[0].factory.clone().unwrap();

    // cw721 code id not in allowed code ids
    let res = bmt.collection_response_vec[0].minter.clone();
    assert!(res.is_none());

    // add cw721_code_id to allowed code ids
    let update_msg = BaseUpdateParamsMsg {
        add_cw721_code_ids: Some(vec![cw721_code_id]),
        rm_cw721_code_ids: None,
        frozen: None,
        code_id: None,
        creation_fee: None,
        min_mint_price: None,
        mint_fee_bps: None,
        max_trading_offset_secs: None,
        extension: None,
    };
    let sudo_msg = SudoMsg::UpdateParams(Box::new(update_msg));
    let res = router.wasm_sudo(factory.clone(), &sudo_msg);
    assert!(res.is_ok());

    let msg = FactoryUtilsQueryMsg::AllowedCollectionCodeIds {};
    let res: AllowedCollectionCodeIdsResponse = router
        .wrap()
        .query_wasm_smart(factory.clone(), &msg)
        .unwrap();
    assert!(res.code_ids.contains(&cw721_code_id));

    // store cw721_base 4-7 code ids
    for _ in 0..(cw721_code_id - 3) {
        router.store_code(contract_cw721_base());
    }

    // create minter with cw721_code_id
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let mut collection_params = mock_collection_params_1(Some(start_time));
    collection_params.code_id = cw721_code_id;

    let msg = BaseMinterCreateMsg {
        init_msg: None,
        collection_params,
    };
    // msg.collection_params.info.creator = creator.to_string();
    let creation_fee = coins(CREATION_FEE, NATIVE_DENOM);
    let msg = FactoryUtilsExecuteMsg::CreateMinter(msg);
    let res = router.execute_contract(creator, factory, &msg, &creation_fee);
    res.unwrap();

    // confirm new cw721 code id == cw721_code_id
    let res = router
        .wrap()
        .query_wasm_contract_info("contract2".to_string())
        .unwrap();
    assert!(res.code_id == cw721_code_id);
}

#[test]
fn test_check_mint() {
    let bmt = base_minter_with_cw721nt(1);
    let (mut router, creator, buyer) = (bmt.router, bmt.accts.creator, bmt.accts.buyer);
    let minter_addr = bmt.collection_response_vec[0].minter.clone().unwrap();
    let collection_addr = bmt.collection_response_vec[0].collection.clone().unwrap();

    // Fail with incorrect token uri
    let mint_msg = ExecuteMsg::Mint {
        token_uri: "test uri".to_string(),
    };
    let err = router.execute_contract(creator.clone(), minter_addr.clone(), &mint_msg, &[]);
    assert!(err.is_err());

    // Fail with incorrect mint price
    let mint_msg = ExecuteMsg::Mint {
        token_uri: "ipfs://example".to_string(),
    };
    let err = router.execute_contract(
        creator.clone(),
        minter_addr.clone(),
        &mint_msg,
        &[coin(MIN_MINT_PRICE + 100, NATIVE_DENOM)],
    );
    assert!(err.is_err());

    // Not authorized to mint
    let mint_msg = ExecuteMsg::Mint {
        token_uri: "ipfs://example".to_string(),
    };
    let err = router.execute_contract(
        buyer,
        minter_addr.clone(),
        &mint_msg,
        &[coin(MIN_MINT_PRICE, NATIVE_DENOM)],
    );
    assert!(err.is_err());

    // Succeeds if funds are sent
    let mint_msg = ExecuteMsg::Mint {
        token_uri: "ipfs://example".to_string(),
    };
    let res = router.execute_contract(
        creator.clone(),
        minter_addr.clone(),
        &mint_msg,
        &[coin(MIN_MINT_PRICE, NATIVE_DENOM)],
    );

    println!("res is {res:?}");
    assert!(res.is_ok());

    let creator_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
    assert_eq!(
        creator_balances,
        coins(
            (INITIAL_BALANCE + CREATION_FEE) - CREATION_FEE - MIN_MINT_PRICE,
            NATIVE_DENOM
        )
    );

    let res: ConfigResponse = router
        .wrap()
        .query_wasm_smart(minter_addr, &QueryMsg::Config {})
        .unwrap();
    assert_eq!(res.collection_address, "contract2".to_string());
    assert_eq!(res.config.mint_price.amount.u128(), MIN_MINT_PRICE);

    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(
            collection_addr.clone(),
            &Cw721QueryMsg::<Empty, Empty, Empty>::OwnerOf {
                token_id: String::from("1"),
                include_expired: None,
            },
        )
        .unwrap();
    assert_eq!(res.owner, creator.to_string());

    // make sure cw721-nt cannot be transferred
    let transfer_msg = Cw721ExecuteMsg::<Empty, Empty, Empty>::TransferNft {
        recipient: "adsf".to_string(),
        token_id: "1".to_string(),
    };
    let err = router.execute_contract(
        creator,
        Addr::unchecked(collection_addr),
        &transfer_msg,
        &[],
    );
    assert!(err.is_err());
}

#[test]
fn update_start_trading_time() {
    let bmt = base_minter_with_cw721(1);
    let (mut router, creator, buyer) = (bmt.router, bmt.accts.creator, bmt.accts.buyer);
    let minter_addr = bmt.collection_response_vec[0].minter.clone().unwrap();
    let collection_addr = bmt.collection_response_vec[0].collection.clone().unwrap();
    let current_block_time = router.block_info().time;

    let default_start_trading_time =
        current_block_time.plus_seconds(mock_params().max_trading_offset_secs + 1);

    // unauthorized
    let res = router.execute_contract(
        Addr::unchecked(buyer),
        Addr::unchecked(minter_addr.clone()),
        &ExecuteMsg::UpdateStartTradingTime(Some(default_start_trading_time)),
        &[],
    );
    assert!(res.is_err());

    // invalid start trading time
    let res = router.execute_contract(
        Addr::unchecked(creator.clone()),
        Addr::unchecked(minter_addr.clone()),
        &ExecuteMsg::UpdateStartTradingTime(Some(Timestamp::from_nanos(0))),
        &[],
    );
    assert!(res.is_err());

    // succeeds
    let res = router.execute_contract(
        Addr::unchecked(creator),
        Addr::unchecked(minter_addr),
        &ExecuteMsg::UpdateStartTradingTime(Some(default_start_trading_time)),
        &[],
    );
    println!("res is {res:?}");
    assert!(res.is_ok());

    // // confirm trading start time
    // let res: CollectionInfoAndExtensionResponse<RoyaltyInfoResponse> = router
    //     .wrap()
    //     .query_wasm_smart(
    //         collection_addr,
    //         &cw721QueryMsg::GetCollectionInfoAndExtension {},
    //     )
    //     .unwrap();
    // assert_eq!(res.start_trading_time, Some(default_start_trading_time));
}
