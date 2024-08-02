use cosmwasm_schema::{cw_serde, QueryResponses};
use cw_ownable::cw_ownable_execute;

use crate::query::{ApprovalResponse, BalanceOfResponse, OwnerOfResponse};

#[cw_serde]
pub struct InstantiateMsg {
    /// Name of the NFT contract
    pub name: String,
    /// Symbol of the NFT contract
    pub symbol: String,

    pub minter: Option<String>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Transfer is a base message to move a token to another account
    TransferNft { recipient: String, token_id: u32 },

    /// Allows spender to transfer the token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    Approve { spender: String, token_id: u32 },

    /// Mint a new NFT, can only be called by the contract minter
    Mint {
        /// Unique ID of the NFT
        token_id: u32,
        /// The owner of the newly minter NFT
        owner: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Return the number of NFTs a user owns
    #[returns(BalanceOfResponse)]
    BalanceOf { owner: String },
    /// Return the owner of the given token, error if token does not exist
    #[returns(OwnerOfResponse)]
    OwnerOf { token_id: u32 },
    /// Return spender that can access all of the owner's tokens.
    #[returns(ApprovalResponse)]
    Approval { token_id: u32 },

    /// Return the minter
    #[returns(MinterResponse)]
    Minter {},
}

/// Shows who can mint these tokens
#[cw_serde]
pub struct MinterResponse {
    pub minter: Option<String>,
}
