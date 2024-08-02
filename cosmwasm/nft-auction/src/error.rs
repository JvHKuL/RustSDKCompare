use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownership(#[from] OwnershipError),

    #[error("auction already started")]
    AlreadyStarted,

    #[error("auction not started")]
    NotStarted,

    #[error("auction already ended")]
    AlreadyEnded,

    #[error("auction bidding ended")]
    BiddingEnded,

    #[error("auction bidding not ended")]
    BiddingNotEnded,

    #[error("auction bid too low")]
    BiddingTooLow,

    #[error("No withdrawable bid for {bidder}")]
    NoWithdrawableBid { bidder: String },
}
