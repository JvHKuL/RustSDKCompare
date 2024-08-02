use crate::state::{Config, NftAuction, Status};
use cosmwasm_std::{
    to_json_binary, BankMsg, Coin, CosmosMsg, DepsMut, Env, Event, MessageInfo, Response, WasmMsg,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};

impl<'a> NftAuction<'a> {
    pub fn instantiate(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        let caller = &info.sender;
        cw_ownable::initialize_owner(deps.storage, deps.api, Some(caller.as_ref()))?;

        let config = Config {
            nft_contract: msg.nft_contract,
            nft_id: msg.nft_id,
            starting_bid: msg.starting_bid.to_owned(),
        };

        self.config.save(deps.storage, &config)?;

        let status = Status {
            started: false,
            ended: false,
            end_at: None,
            highest_bidder: None,
            highest_bid: msg.starting_bid.to_owned(),
        };
        self.status.save(deps.storage, &status)?;

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
            ExecuteMsg::Start {} => self.start(deps, env, info),
            ExecuteMsg::Bid {} => self.bid(deps, env, info),
            ExecuteMsg::Withdraw {} => self.withdraw(deps, info),
            ExecuteMsg::End {} => self.end(deps, env, info),
        }
    }

    fn start(&self, deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
        // Start must be called by the owner of this auction
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

        // Get Contract Status
        let mut status = self.status.load(deps.storage)?;

        if status.started {
            return Err(ContractError::AlreadyStarted);
        }

        status.started = true;
        status.end_at = Some(env.block.time.plus_seconds(5 * 60));
        self.status.save(deps.storage, &status)?;

        let Config {
            nft_contract,
            nft_id,
            ..
        } = self.config.load(deps.storage)?;

        // Response message: transfer NFT from sender to this contract + nft ID
        let erc_transfer_msg = erc721::ExecuteMsg::TransferNft {
            recipient: env.contract.address.to_string(),
            token_id: nft_id,
        };
        let erc_transfer_msg = WasmMsg::Execute {
            contract_addr: nft_contract.into_string(),
            msg: to_json_binary(&erc_transfer_msg)?,
            funds: vec![],
        };

        let event = Event::new("start")
            .add_attribute("action", "start")
            .add_attribute("caller", info.sender.as_str())
            .add_attribute("end_at", status.end_at.unwrap().to_string());

        let resp = Response::new()
            .add_message(erc_transfer_msg)
            .add_event(event);

        Ok(resp)
    }

    fn bid(&self, deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
        let mut status = self.status.load(deps.storage)?;
        // Check auction already started
        if !status.started {
            return Err(ContractError::NotStarted);
        }

        // Check auction not yet over time
        let now = env.block.time;
        if status.ended || now >= status.end_at.unwrap() {
            return Err(ContractError::BiddingEnded);
        }

        // Check bid higher than current bid
        let coin = info.funds.iter().find(|coin| {
            coin.denom == status.highest_bid.denom && coin.amount > status.highest_bid.amount
        });

        if coin == None {
            return Err(ContractError::BiddingTooLow);
        }

        // If there is already a highest bidder, it will be added to the list of bids
        if let Some(prev_highest_addr) = status.highest_bidder {
            let prev_highest = self.bids.may_load(deps.storage, &prev_highest_addr)?;
            match prev_highest {
                Some(prev_coin) => {
                    let total_bid = Coin::new(
                        (prev_coin.amount + status.highest_bid.amount).u128(),
                        prev_coin.denom,
                    );
                    self.bids
                        .save(deps.storage, &prev_highest_addr, &total_bid)?;
                }
                None => self
                    .bids
                    .save(deps.storage, &prev_highest_addr, &status.highest_bid)?,
            }
        }

        status.highest_bidder = Some(info.sender.to_owned());
        status.highest_bid = coin.unwrap().to_owned();
        self.status.save(deps.storage, &status)?;

        let resp = Response::new()
            .add_attribute("action", "bid")
            .add_attribute("bidder", info.sender.as_str())
            .add_attribute(
                "value",
                coin.unwrap().amount.to_string() + " " + &coin.unwrap().denom,
            );

        Ok(resp)
    }

    fn withdraw(&self, deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
        let caller = info.sender;
        let bid = self.bids.may_load(deps.storage, &caller)?;
        match bid {
            Some(coin) => {
                // Add bank transfer to message
                let bank_msg = BankMsg::Send {
                    to_address: caller.to_string(),
                    amount: vec![coin.to_owned()],
                };

                let resp = Response::new()
                    .add_message(bank_msg)
                    .add_attribute("action", "withdraw")
                    .add_attribute("bidder", caller.as_str())
                    .add_attribute("value", coin.amount.to_string() + " " + &coin.denom);

                Ok(resp)
            }
            None => Err(ContractError::NoWithdrawableBid {
                bidder: caller.to_string(),
            }),
        }
    }

    fn end(&self, deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
        let caller = info.sender;
        let mut status = self.status.load(deps.storage)?;
        // Check auction already started
        if !status.started {
            return Err(ContractError::NotStarted);
        }

        if status.ended {
            return Err(ContractError::AlreadyEnded);
        }

        let now = env.block.time;
        if now < status.end_at.unwrap() {
            return Err(ContractError::BiddingNotEnded);
        }

        status.ended = true;

        let config = self.config.load(deps.storage)?;
        self.status.save(deps.storage, &status)?;

        let mut msgs: Vec<CosmosMsg> = Vec::new();
        let seller = cw_ownable::get_ownership(deps.storage)
            .unwrap()
            .owner
            .unwrap();
        match status.highest_bidder.to_owned() {
            Some(bidder) => {
                // Send NFT to bidder and bid to seller
                let erc_transfer_msg = erc721::ExecuteMsg::TransferNft {
                    recipient: bidder.to_string(),
                    token_id: config.nft_id,
                };
                let erc_transfer_msg = WasmMsg::Execute {
                    contract_addr: config.nft_contract.into_string(),
                    msg: to_json_binary(&erc_transfer_msg)?,
                    funds: vec![],
                };
                msgs.push(erc_transfer_msg.into());

                // Add bank transfer to seller
                let bank_msg = BankMsg::Send {
                    to_address: seller.to_string(),
                    amount: vec![status.highest_bid.to_owned()],
                };
                msgs.push(bank_msg.into());
            }
            None => {
                // Send NFT back to seller
                let erc_transfer_msg = erc721::ExecuteMsg::TransferNft {
                    recipient: seller.to_string(),
                    token_id: config.nft_id,
                };
                let erc_transfer_msg = WasmMsg::Execute {
                    contract_addr: config.nft_contract.into_string(),
                    msg: to_json_binary(&erc_transfer_msg)?,
                    funds: vec![],
                };
                msgs.push(erc_transfer_msg.into());
            }
        }

        let resp = Response::new()
            .add_attribute("action", "end")
            .add_messages(msgs)
            .add_attribute("caller", caller.as_str())
            .add_attribute("winner", status.highest_bidder.unwrap_or(caller).as_str())
            .add_attribute(
                "value",
                status.highest_bid.amount.to_string() + " " + &status.highest_bid.denom,
            );
        // Optimization: send money back to bidders automatically

        Ok(resp)
    }
}
