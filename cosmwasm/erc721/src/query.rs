use cosmwasm_schema::cw_serde;

use cosmwasm_std::{to_json_binary, Binary, Deps, Env, StdResult};

use crate::msg::{MinterResponse, QueryMsg};
use crate::state::Erc721;

impl<'a> Erc721<'a> {
    pub fn query(&self, deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::Minter {} => to_json_binary(&self.minter(deps)?),
            QueryMsg::BalanceOf { owner } => to_json_binary(&self.balance_of(deps, env, owner)?),
            QueryMsg::OwnerOf { token_id } => to_json_binary(&self.owner_of(deps, env, token_id)?),
            QueryMsg::Approval { token_id } => to_json_binary(&self.approval(deps, env, token_id)?),
        }
    }

    fn minter(&self, deps: Deps) -> StdResult<MinterResponse> {
        let minter = cw_ownable::get_ownership(deps.storage)?
            .owner
            .map(|a| a.into_string());

        Ok(MinterResponse { minter })
    }

    fn balance_of(&self, deps: Deps, _env: Env, of: String) -> StdResult<BalanceOfResponse> {
        let of_addr = deps.api.addr_validate(&of)?;
        let balance = self.owned_tokens_count.may_load(deps.storage, &of_addr)?;

        let count = match balance {
            Some(x) => x,
            None => 0,
        };

        Ok(BalanceOfResponse { owner: of, count })
    }

    fn owner_of(&self, deps: Deps, _env: Env, token_id: u32) -> StdResult<OwnerOfResponse> {
        let owner = self.token_owner.load(deps.storage, token_id)?;

        Ok(OwnerOfResponse { owner })
    }

    fn approval(&self, deps: Deps, _env: Env, token_id: u32) -> StdResult<ApprovalResponse> {
        let approved = self.token_approvals.may_load(deps.storage, token_id)?;

        Ok(ApprovalResponse { approver: approved })
    }
}

#[cw_serde]
pub struct BalanceOfResponse {
    /// Owner of the tokens
    pub owner: String,
    /// Token count
    pub count: u32,
}

#[cw_serde]
pub struct OwnerOfResponse {
    /// Owner of the token
    pub owner: String,
}

#[cw_serde]
pub struct ApprovalResponse {
    pub approver: Option<String>,
}
