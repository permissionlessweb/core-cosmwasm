use cosmwasm_std::Addr;


pub struct InstantiateParams<'a> {
    pub addresses: Vec<String>,
    pub funds_amount: u128,
    pub expected_airdrop_contract_id: u64,
    pub minter_address: Addr,
    pub admin_account: Addr,
    pub app: &'a mut TerpApp,
    pub per_address_limit: u32,
    pub claim_msg_plaintext: String,
}
