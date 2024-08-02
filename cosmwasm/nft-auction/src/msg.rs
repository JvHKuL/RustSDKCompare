use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};

#[cw_serde]
pub struct InstantiateMsg {
    /// Name of the NFT contract
    pub nft_contract: Addr,
    /// Symbol of the NFT contract
    pub nft_id: u32,

    pub starting_bid: Coin,
}

#[cw_serde]
pub enum ExecuteMsg {
    Start {},
    Bid {},
    Withdraw {},
    End {},
}

#[cw_serde]
pub enum QueryMsg {
    Dummy {},
}
