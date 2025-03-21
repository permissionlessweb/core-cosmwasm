#[cfg(test)]
mod tests {
    use cosmwasm_std::Addr;
    use cw_multi_test::{App, Executor};
    
    use vending_factory::helpers::FactoryContract;
    use vending_factory::msg::InstantiateMsg;

    use crate::common_setup::contract_boxes::{contract_vending_factory, custom_mock_app};
    use crate::common_setup::setup_minter::vending_minter::mock_params::mock_params;

    const GOVERNANCE: &str = "governance";

    fn proper_instantiate() -> (App, FactoryContract) {
        let mut app = custom_mock_app();
        let factory_id = app.store_code(contract_vending_factory());
        let minter_id = 2;

        let mut params = mock_params(None);
        params.code_id = minter_id;

        let factory_contract_addr = app
            .instantiate_contract(
                factory_id,
                Addr::unchecked(GOVERNANCE),
                &InstantiateMsg { params },
                &[],
                "factory",
                None,
            )
            .unwrap();

        (app, FactoryContract(factory_contract_addr))
    }

    mod init {
        use super::*;

        #[test]
        fn can_init() {
            let (_, factory_contract) = proper_instantiate();
            assert_eq!(factory_contract.addr().to_string(), "contract0");
        }
    }
}
