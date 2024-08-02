use cw_ownable::OwnershipError;

use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::Erc721;

impl<'a> Erc721<'a> {
    pub fn instantiate(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        let owner = match msg.minter {
            Some(owner) => deps.api.addr_validate(&owner)?,
            None => info.sender,
        };
        cw_ownable::initialize_owner(deps.storage, deps.api, Some(owner.as_ref()))?;

        Ok(Response::default())
    }

    pub fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        match msg {
            ExecuteMsg::Mint { token_id, owner } => self.mint(deps, info, token_id, owner),
            ExecuteMsg::Approve { spender, token_id } => {
                self.approve(deps, env, info, spender, token_id)
            }
            ExecuteMsg::TransferNft {
                recipient,
                token_id,
            } => self.transfer_nft(deps, env, info, recipient, token_id),
            ExecuteMsg::UpdateOwnership(action) => Self::update_ownership(deps, env, info, action),
        }
    }

    fn mint(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        token_id: u32,
        owner: String,
    ) -> Result<Response, ContractError> {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

        let token_owner = self.token_owner.may_load(deps.storage, token_id)?;

        if let Some(_) = token_owner {
            return Err(ContractError::Claimed);
        }

        let owner_addr = deps.api.addr_validate(&owner)?;

        self.owned_tokens_count
            .update(deps.storage, &owner_addr, |old| match old {
                Some(x) => Ok::<u32, ContractError>(x + 1),
                None => Ok(1),
            })?;
        let _ = self.token_owner.save(deps.storage, token_id, &owner)?;

        Ok(Response::new()
            .add_attribute("action", "mint")
            .add_attribute("minter", info.sender)
            .add_attribute("owner", owner)
            .add_attribute("token_id", token_id.to_string()))
    }

    fn approve(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        spender: String,
        token_id: u32,
    ) -> Result<Response, ContractError> {
        let caller = &info.sender;
        let owner = self.token_owner.may_load(deps.storage, token_id)?;
        match owner {
            Some(owner) => {
                if deps.api.addr_validate(&owner)? != caller {
                    return Err(ContractError::Ownership(OwnershipError::NotOwner));
                }
            }
            None => return Err(ContractError::Ownership(OwnershipError::NoOwner)),
        }

        let _spender_addr = deps.api.addr_validate(&spender)?;

        let _ = self
            .token_approvals
            .save(deps.storage, token_id, &spender)?;

        Ok(Response::new()
            .add_attribute("action", "approve")
            .add_attribute("sender", &info.sender)
            .add_attribute("spender", &spender)
            .add_attribute("token_id", token_id.to_string()))
    }

    fn transfer_nft(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        recipient: String,
        token_id: u32,
    ) -> Result<Response, ContractError> {
        let caller = &info.sender;
        let owner = self.token_owner.may_load(deps.storage, token_id)?;
        let owner_addr = match owner {
            Some(owner) => deps.api.addr_validate(&owner)?,
            None => return Err(ContractError::Ownership(OwnershipError::NoOwner)),
        };
        if !(owner_addr == caller
            || self.token_approvals.may_load(deps.storage, token_id)? == Some(caller.to_string()))
        {
            return Err(ContractError::ApprovalNotFound {
                spender: caller.to_string(),
            });
        };

        // Check recipient is valid address.
        let recipient_addr = deps.api.addr_validate(&recipient)?;

        self.token_approvals.remove(deps.storage, token_id);
        let _ = self
            .owned_tokens_count
            .update(deps.storage, &owner_addr, |old| match old {
                Some(x) => Ok(x - 1),
                None => Err(ContractError::Std(cosmwasm_std::StdError::GenericErr {
                    msg: "Should not be possible".to_string(),
                })),
            });
        self.token_owner.remove(deps.storage, token_id);

        let _ = self.token_owner.save(deps.storage, token_id, &recipient)?;
        self.owned_tokens_count
            .update(deps.storage, &recipient_addr, |old| match old {
                Some(x) => Ok::<u32, ContractError>(x + 1),
                None => Ok(1),
            })?;

        Ok(Response::new()
            .add_attribute("action", "transfer_nft")
            .add_attribute("sender", info.sender)
            .add_attribute("recipient", recipient)
            .add_attribute("token_id", token_id.to_string()))
    }

    fn update_ownership(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        action: cw_ownable::Action,
    ) -> Result<Response, ContractError> {
        let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
        Ok(Response::new().add_attributes(ownership.into_attributes()))
    }
}
