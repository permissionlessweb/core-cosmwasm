use crate::common_setup::templates::vending_minter_template;
use crate::common_setup::{
    contract_boxes::contract_terp721_base, setup_minter::common::constants::CREATION_FEE,
    setup_minter::vending_minter::mock_params::mock_init_extension,
    templates::vending_minter_with_specified_terp721,
};
use cosmwasm_std::{coins, Timestamp};
use cw_multi_test::Executor;
use factory_utils::{
    msg::FactoryUtilsExecuteMsg,
    query::{AllowedCollectionCodeIdsResponse, FactoryUtilsQueryMsg},
    tests::mock_collection_params_1,
};
use terp_sdk::{GENESIS_MINT_START_TIME, NATIVE_DENOM};
use vending_factory::msg::{
    SudoMsg, VendingMinterCreateMsg, VendingUpdateParamsExtension, VendingUpdateParamsMsg,
};

#[test]
fn init() {
    let vt = vending_minter_template(2);
    vt.collection_response_vec[0].minter.clone().unwrap();
}

#[test]
fn update_code_id() {
    let terp721_code_id = 7u64;

    let vt = vending_minter_with_specified_terp721(200, terp721_code_id);
    let (mut router, creator, _) = (vt.router, vt.accts.creator, vt.accts.buyer);
    let factory = vt.collection_response_vec[0].factory.clone().unwrap();

    // terp721 code id not in allowed code ids
    let res = vt.collection_response_vec[0].minter.clone();
    assert!(res.is_none());

    // add terp721_code_id to allowed code ids
    let extension = VendingUpdateParamsExtension {
        max_token_limit: None,
        max_per_address_limit: None,
        airdrop_mint_price: None,
        airdrop_mint_fee_bps: None,
        shuffle_fee: None,
    };
    let update_msg = VendingUpdateParamsMsg {
        add_terp721_code_ids: Some(vec![terp721_code_id]),
        rm_terp721_code_ids: None,
        frozen: None,
        code_id: None,
        creation_fee: None,
        min_mint_price: None,
        mint_fee_bps: None,
        max_trading_offset_secs: None,
        extension,
    };
    let sudo_msg = SudoMsg::UpdateParams(Box::new(update_msg));
    let res = router.wasm_sudo(factory.clone(), &sudo_msg);
    assert!(res.is_ok());

    let msg = FactoryUtilsQueryMsg::AllowedCollectionCodeIds {};
    let res: AllowedCollectionCodeIdsResponse = router
        .wrap()
        .query_wasm_smart(factory.clone(), &msg)
        .unwrap();
    assert!(res.code_ids.contains(&terp721_code_id));

    // store terp721_base 4-7 code ids
    for _ in 0..(terp721_code_id - 3) {
        router.store_code(contract_terp721_base());
    }

    // create minter with terp721_code_id
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let mut collection_params = mock_collection_params_1(Some(start_time));
    collection_params.code_id = terp721_code_id;

    let init_msg = mock_init_extension(None, None);
    let mut msg = VendingMinterCreateMsg {
        init_msg,
        collection_params,
    };
    msg.collection_params.info.creator = creator.to_string();
    let creation_fee = coins(CREATION_FEE, NATIVE_DENOM);
    let msg = FactoryUtilsExecuteMsg::CreateMinter(msg);
    let res = router.execute_contract(creator, factory, &msg, &creation_fee);
    assert!(res.is_ok());

    // confirm new terp721 code id == terp721_code_id
    let res = router
        .wrap()
        .query_wasm_contract_info("contract2".to_string())
        .unwrap();
    assert!(res.code_id == terp721_code_id);
}
