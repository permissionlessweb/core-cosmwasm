use cosmwasm_std::Empty;
use cw_storage_plus::Item;
use factory_utils::MinterParams;

pub type Extension = Option<Empty>;
pub type BaseMinterParams = MinterParams<Extension>;

pub const SUDO_PARAMS: Item<BaseMinterParams> = Item::new("sudo-params");
