use cosmwasm_std::{Addr, Decimal, Empty};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

pub fn contract_residual_registry() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        terp_residual_registry::execute::execute,
        terp_residual_registry::instantiate::instantiate,
        terp_residual_registry::query::query,
    )
    .with_sudo(terp_residual_registry::sudo::sudo);
    Box::new(contract)
}

pub fn setup_residual_registry(router: &mut App, creator: Addr) -> Addr {
    let residual_registry_id = router.store_code(contract_residual_registry());
    let msg = terp_residual_registry::msg::InstantiateMsg {
        config: terp_residual_registry::state::Config {
            update_wait_period: 6,
            max_share_delta: Decimal::percent(1),
        },
    };
    router
        .instantiate_contract(
            residual_registry_id,
            creator,
            &msg,
            &[],
            "terp_residual_registry",
            None,
        )
        .unwrap()
}
