use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Timestamp};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub nft_contract: Addr,

    pub nft_id: u32,

    pub starting_bid: Coin,
}

#[cw_serde]
pub struct Status {
    pub started: bool,
    pub ended: bool,
    pub end_at: Option<Timestamp>,
    pub highest_bidder: Option<Addr>,
    pub highest_bid: Coin,
}

pub struct NftAuction<'a> {
    pub config: Item<'a, Config>,
    pub status: Item<'a, Status>,
    pub bids: Map<'a, &'a Addr, Coin>,
}

impl Default for NftAuction<'static> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> NftAuction<'a> {
    fn new() -> Self {
        Self {
            config: Item::new("config"),
            status: Item::new("status"),
            bids: Map::new("bids"),
        }
    }
}
