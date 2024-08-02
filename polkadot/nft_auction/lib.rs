#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod nft_auction {
    use ink::env::{
        call::{build_call, ExecutionInput, Selector},
        DefaultEnvironment,
    };
    use ink::storage::Mapping;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Return if the balance cannot fulfill a request.
        AlreadyStarted,
        AlreadyEnded,
        BiddingEnded,
        BiddingNotEnded,
        BidTooLow,
        NotSeller,
        NotStarted,
    }

    #[ink(event)]
    pub struct Start {
        #[ink(topic)]
        caller: AccountId,
        end_at: Timestamp,
    }

    #[ink(event)]
    pub struct Bid {
        #[ink(topic)]
        bidder: AccountId,
        value: Balance,
    }

    #[ink(event)]
    pub struct Withdraw {
        #[ink(topic)]
        bidder: AccountId,
        value: Balance,
    }

    #[ink(event)]
    pub struct End {
        #[ink(topic)]
        caller: AccountId,
        winner: AccountId,
        value: Balance,
    }

    pub type TokenId = u32;
    pub type Result<T> = core::result::Result<T, Error>;

    const DURATION: u64 = 5 * 60 * 1000; // in milliseconds

    #[ink(storage)]
    pub struct NftAuction {
        started: bool,
        ended: bool,
        seller: AccountId,
        end_at: Option<Timestamp>,
        highest_bidder: Option<AccountId>,
        highest_bid: Balance,
        bids: Mapping<AccountId, Balance>,
        nft: AccountId,
        nft_id: TokenId,
    }

    impl NftAuction {
        #[ink(constructor)]
        pub fn new(nft: AccountId, nft_id: TokenId, starting_bid: Balance) -> Self {
            Self {
                started: false,
                ended: false,
                seller: Self::env().caller(),
                end_at: None,
                highest_bidder: None,
                highest_bid: starting_bid,
                bids: Mapping::default(),
                nft,
                nft_id,
            }
        }

        #[ink(message)]
        #[allow(clippy::arithmetic_side_effects)]
        pub fn start(&mut self) -> Result<()> {
            let caller = self.env().caller();
            if self.started {
                return Err(Error::AlreadyStarted);
            }
            if self.seller != caller {
                return Err(Error::NotSeller);
            }

            // https://use.ink/basics/cross-contract-calling/
            // https://docs.alephzero.org/aleph-zero/build/cross-contract-calls/using-dynamic-calls
            let _my_return_value = build_call::<DefaultEnvironment>()
                .call(self.nft) //Contract address
                .call_v1()
                .gas_limit(0)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("transfer_from")))
                        .push_arg(self.seller) //sender
                        .push_arg(self.env().account_id()) //address(this)
                        .push_arg(self.nft_id), //nftId
                )
                .returns::<ink::MessageResult<()>>()
                .invoke();

            self.started = true;
            let now = self.env().block_timestamp();
            self.end_at = Some(now + DURATION);

            self.env().emit_event(Start {
                caller,
                end_at: self.end_at.unwrap(),
            });

            Ok(())
        }

        #[ink(message, payable)]
        #[allow(clippy::arithmetic_side_effects)]
        pub fn bid(&mut self) -> Result<()> {
            let caller = self.env().caller();
            if !self.started {
                return Err(Error::NotStarted);
            }

            let now = self.env().block_timestamp();
            if now >= self.end_at.unwrap() {
                return Err(Error::BiddingEnded);
            }

            if self.env().transferred_value() <= self.highest_bid {
                return Err(Error::BidTooLow);
            }

            // A previous bidder should best withdraw before bidding again
            if let Some(b) = self.highest_bidder {
                self.bids.insert(
                    b,
                    &(self.bids.get(b).unwrap_or(0) + self.env().transferred_value()),
                );
            }

            self.highest_bidder = Some(caller);
            self.highest_bid = self.env().transferred_value();

            self.env().emit_event(Bid {
                bidder: caller,
                value: self.env().transferred_value(),
            });
            Ok(())
        }

        #[ink(message)]
        pub fn withdraw(&mut self) -> Result<()> {
            let caller = self.env().caller();
            if self.bids.contains(caller) {
                let bal = self.bids.get(caller).unwrap();
                self.bids.insert(caller, &0);

                if self.env().transfer(caller, bal).is_err() {
                    panic!(
                        "requested transfer failed. this can be the case if the contract does not\
                         have sufficient free funds or if the transfer would have brought the\
                         contract's balance below minimum balance."
                    )
                }

                self.env().emit_event(Withdraw {
                    bidder: caller,
                    value: bal,
                });
            }

            Ok(())
        }

        #[ink(message)]
        pub fn end(&mut self) -> Result<()> {
            if !self.started {
                return Err(Error::NotStarted);
            }
            if self.ended {
                return Err(Error::AlreadyEnded);
            }
            let now = self.env().block_timestamp();
            if now < self.end_at.unwrap() {
                return Err(Error::BiddingNotEnded);
            }

            self.ended = true;

            match self.highest_bidder {
                Some(b) => {
                    // Send NFT to bidder and bid to seller
                    let _my_return_value = build_call::<DefaultEnvironment>()
                        .call(self.nft) //Contract address
                        .call_v1()
                        .gas_limit(0)
                        .exec_input(
                            ExecutionInput::new(Selector::new(ink::selector_bytes!(
                                "transfer_from"
                            )))
                            .push_arg(self.env().account_id())
                            .push_arg(b) //highest bidder
                            .push_arg(self.nft_id),
                        )
                        .returns::<ink::MessageResult<()>>()
                        .invoke();

                    if self.env().transfer(self.seller, self.highest_bid).is_err() {
                        panic!(
                            "requested transfer failed. this can be the case if the contract does not\
                             have sufficient free funds or if the transfer would have brought the\
                             contract's balance below minimum balance."
                        )
                    }

                    return Err(Error::BidTooLow);
                }
                None => {
                    // Send NFT back to seller
                    let _my_return_value = build_call::<DefaultEnvironment>()
                        .call(self.nft)
                        .call_v1()
                        .gas_limit(0)
                        .exec_input(
                            ExecutionInput::new(Selector::new(ink::selector_bytes!(
                                "transfer_from"
                            )))
                            .push_arg(self.env().account_id())
                            .push_arg(self.seller)
                            .push_arg(self.nft_id),
                        )
                        .returns::<ink::MessageResult<()>>()
                        .invoke();
                }
            }

            self.env().emit_event(End {
                caller: self.env().caller(),
                winner: self.highest_bidder.unwrap_or(self.env().caller()),
                value: self.highest_bid,
            });

            Ok(())
        }
    }
}
