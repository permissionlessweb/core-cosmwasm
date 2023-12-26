use crate::setup::setup_contracts::setup_residual_registry;
use crate::setup::setup_minter::standard_minter_template;

use cosmwasm_std::Decimal;
use terp_residual_registry::{
    msg::{QueryMsg, SudoMsg},
    state::Config,
};

#[test]
fn try_sudo_update_config() {
    let vt = standard_minter_template(1);
    let (mut router, creator, _bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let residual_registry = setup_residual_registry(&mut router, creator);

    let new_config = Config {
        update_wait_period: 10,
        max_share_delta: Decimal::percent(5),
    };

    let sudo_msg = SudoMsg::UpdateConfig {
        config: new_config.clone(),
    };

    let response = router.wasm_sudo(residual_registry.clone(), &sudo_msg);
    assert!(response.is_ok());

    let config: Config = router
        .wrap()
        .query_wasm_smart(&residual_registry, &QueryMsg::Config {})
        .unwrap();

    assert_eq!(config, new_config);
}
