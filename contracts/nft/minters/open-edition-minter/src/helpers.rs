use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, Coin, ContractInfoResponse, CosmosMsg, CustomQuery, Querier,
    QuerierWrapper, StdError, StdResult, WasmMsg, WasmQuery,
};
use cw721_base::msg::ExecuteMsg as cw721ExecuteMsg;
use terp_metadata::Metadata;

use crate::msg::{ConfigResponse, ExecuteMsg, QueryMsg};

/// MinterContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[cw_serde]
pub struct MinterContract(pub Addr);

impl MinterContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_json_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }

    pub fn call_with_funds<T: Into<ExecuteMsg>>(
        &self,
        msg: T,
        funds: Coin,
    ) -> StdResult<CosmosMsg> {
        let msg = to_json_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![funds],
        }
        .into())
    }

    pub fn contract_info<Q, T, CQ>(&self, querier: &Q) -> StdResult<ContractInfoResponse>
    where
        Q: Querier,
        T: Into<String>,
        CQ: CustomQuery,
    {
        let query = WasmQuery::ContractInfo {
            contract_addr: self.addr().into(),
        }
        .into();
        let res: ContractInfoResponse = QuerierWrapper::<CQ>::new(querier).query(&query)?;
        Ok(res)
    }

    pub fn config(&self, querier: &QuerierWrapper) -> StdResult<ConfigResponse> {
        let res: ConfigResponse = querier.query_wasm_smart(self.addr(), &QueryMsg::Config {})?;
        Ok(res)
    }
}

pub fn mint_nft_msg(
    cw721_address: Addr,
    token_id: String,
    recipient_addr: Addr,
    extension: Option<Metadata>,
    token_uri: Option<String>,
) -> Result<CosmosMsg, StdError> {
    let mint_msg = if let Some(_extension) = extension {
        return Err(StdError::generic_err("unimplemeneted"));
        // CosmosMsg::Wasm(WasmMsg::Execute {
        //     contract_addr: cw721_address.to_string(),
        //     msg: to_json_binary(&cw721ExecuteMsg::Mint {
        //         token_id,
        //         owner: recipient_addr.to_string(),
        //         token_uri: None,
        //         extension: Some(extension),
        //     })?,
        //     funds: vec![],
        // })
    } else {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cw721_address.to_string(),
            msg: to_json_binary(&cw721ExecuteMsg::Mint {
                token_id,
                owner: recipient_addr.to_string(),
                token_uri,
                extension: None,
            })?,
            funds: vec![],
        })
    };
    Ok(mint_msg)
}
