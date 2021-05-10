use anyhow::Result;
use contracts::EasyAuction;
use ethabi::ParamType;
use ethcontract::Address;
use ethcontract::BlockNumber;
use model::auction_details::AuctionDetails;
use model::order::Order;
use model::order::OrderWithAuctionId;
use model::user::User;
use primitive_types::{H160, U256};
use std::convert::TryInto;
use tracing::info;
use web3::Web3;

pub struct EventReader {
    pub contract: EasyAuction,
    pub web3: Web3<web3::transports::Http>,
}

pub struct OrderUpdates {
    pub orders_added: Vec<OrderWithAuctionId>,
    pub orders_removed: Vec<OrderWithAuctionId>,
    pub orders_claimed: Vec<OrderWithAuctionId>,
    pub users_added: Vec<User>,
    pub last_block_handled: u64,
}

pub struct DataFromEvent {
    pub order: Order,
    pub timestamp: u64,
}

const BLOCK_CONFIRMATION_COUNT: u64 = 10;
const NUMBER_OF_BLOCKS_TO_SYNC_PER_REQUEST: u64 = 10000;

impl EventReader {
    pub fn new(contract: EasyAuction, web3: Web3<web3::transports::Http>) -> Self {
        Self { contract, web3 }
    }

    pub async fn get_order_updates(&self, from_block: u64, to_block: u64) -> Result<OrderUpdates> {
        let orders_added = self
            .get_order_placements_between_blocks(from_block, to_block)
            .await?;
        let orders_removed = self
            .get_cancellations_between_blocks(from_block, to_block)
            .await?;
        let orders_claimed = self
            .get_order_claims_between_blocks(from_block, to_block)
            .await?;
        let users_added = self
            .get_new_users_between_blocks(from_block, to_block)
            .await?;
        Ok(OrderUpdates {
            orders_added,
            orders_removed,
            orders_claimed,
            users_added,
            last_block_handled: to_block,
        })
    }
    pub async fn get_auction_updates(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<AuctionDetails>> {
        let mut new_auction = Vec::new();
        let events = self
            .contract
            .events()
            .new_auction()
            .from_block(BlockNumber::Number(from_block.into()))
            .to_block(BlockNumber::Number(to_block.into()))
            .query()
            .await?;
        for event in events {
            let mut event_timestamp: Option<u64> = None;
            if let Some(event_meta_data) = event.meta.clone() {
                let block_id = web3::types::BlockId::from(event_meta_data.block_hash);
                let block_info = self.web3.eth().block(block_id).await?;
                if let Some(block_data) = block_info {
                    event_timestamp = Some(block_data.timestamp.as_u64());
                } else {
                    tracing::error!("Unable to retrieve auction starting point");
                };
            } else {
                tracing::error!("Unable to retrieve auction starting point");
            };
            let order = Order {
                sell_amount: U256::from(event.data.auctioned_sell_amount),
                buy_amount: U256::from(event.data.min_buy_amount),
                user_id: 0_u64, // todo: set correctly
            };
            let address_auctioning_token: Address = event.data.auctioning_token;
            let address_bidding_token: Address = event.data.bidding_token;
            let bidding_erc20_contract = contracts::ERC20::at(&self.web3, address_bidding_token);
            let auctioning_erc20_contract =
                contracts::ERC20::at(&self.web3, address_auctioning_token);
            let symbol_auctioning_token = auctioning_erc20_contract.symbol().call().await?;
            let auction_details_from_rpc_call = self
                .contract
                .auction_data(event.data.auction_id)
                .call()
                .await?;
            let is_atomic_closure_allowed = auction_details_from_rpc_call.11;
            let decimals_auctioning_token =
                U256::from(auctioning_erc20_contract.decimals().call().await?);
            let symbol_bidding_token = bidding_erc20_contract.symbol().call().await?;
            let decimals_bidding_token =
                U256::from(bidding_erc20_contract.decimals().call().await?);
            let price_point = order
                .to_price_point(decimals_bidding_token, decimals_auctioning_token)
                .invert_price();
            let mut is_private_auction = true;
            let allow_list_signer: Address = get_address_from_bytes(event.data.allow_list_data);
            if event.data.allow_list_contract == H160::from([0u8; 20]) {
                is_private_auction = false;
            }
            let chain_id = &self.web3.eth().chain_id().await?;
            new_auction.push(AuctionDetails {
                auction_id: event.data.auction_id.as_u64(),
                order: price_point,
                exact_order: order,
                symbol_auctioning_token,
                symbol_bidding_token,
                address_bidding_token,
                address_auctioning_token,
                decimals_auctioning_token,
                decimals_bidding_token,
                minimum_bidding_amount_per_order: event.data.minimum_bidding_amount_per_order,
                min_funding_threshold: event.data.min_funding_threshold,
                allow_list_manager: event.data.allow_list_contract,
                allow_list_signer,
                order_cancellation_end_date: event.data.order_cancellation_end_date.as_u64(),
                end_time_timestamp: event.data.auction_end_date.as_u64(),
                starting_timestamp: event_timestamp.unwrap_or(0_u64),
                current_clearing_price: price_point.price,
                current_bidding_amount: 0_u64,
                is_private_auction,
                is_atomic_closure_allowed,
                chain_id: *chain_id,
                interest_score: 0_f64,
            });
        }
        Ok(new_auction)
    }

    async fn get_order_placements_between_blocks(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<OrderWithAuctionId>> {
        let mut order_updates = Vec::new();
        let events = self
            .contract
            .events()
            .new_sell_order()
            .from_block(BlockNumber::Number(from_block.into()))
            .to_block(BlockNumber::Number(to_block.into()))
            .query()
            .await?;
        for event in events {
            let order = Order {
                sell_amount: U256::from(event.data.sell_amount),
                buy_amount: U256::from(event.data.buy_amount),
                user_id: event.data.user_id as u64,
            };
            let order_update = OrderWithAuctionId {
                auction_id: event.data.auction_id.as_u64(),
                order,
            };
            order_updates.push(order_update);
        }
        Ok(order_updates)
    }

    async fn get_order_claims_between_blocks(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<OrderWithAuctionId>> {
        let mut order_updates = Vec::new();
        let events = self
            .contract
            .events()
            .claimed_from_order()
            .from_block(BlockNumber::Number(from_block.into()))
            .to_block(BlockNumber::Number(to_block.into()))
            .query()
            .await?;
        for event in events {
            let order = Order {
                sell_amount: U256::from(event.data.sell_amount),
                buy_amount: U256::from(event.data.buy_amount),
                user_id: event.data.user_id as u64,
            };
            let order_update = OrderWithAuctionId {
                auction_id: event.data.auction_id.as_u64(),
                order,
            };
            order_updates.push(order_update);
        }
        Ok(order_updates)
    }

    async fn get_new_users_between_blocks(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<User>> {
        let mut users = Vec::new();
        let events = self
            .contract
            .events()
            .new_user()
            .from_block(BlockNumber::Number(from_block.into()))
            .to_block(BlockNumber::Number(to_block.into()))
            .query()
            .await?;
        for event in events {
            let user = User {
                address: event.data.user_address,
                user_id: event.data.user_id,
            };
            users.push(user);
        }
        Ok(users)
    }

    pub async fn get_to_block(
        &self,
        last_handled_block: u64,
        reorg_protection: bool,
    ) -> Result<(u64, u64)> {
        let current_block = self.web3.eth().block_number().await?.as_u64();
        let mut to_block = current_block;
        if reorg_protection {
            to_block -= BLOCK_CONFIRMATION_COUNT;
        }
        let from_block = last_handled_block + 1;
        if from_block > to_block {
            anyhow::bail!("Benign interruption: from_block > to_block for updating events")
        }
        if from_block + NUMBER_OF_BLOCKS_TO_SYNC_PER_REQUEST < to_block {
            to_block = std::cmp::min(to_block, from_block + NUMBER_OF_BLOCKS_TO_SYNC_PER_REQUEST);
        }
        info!(
            "Updating event based orderbook from block {} to block {} ",
            from_block, to_block,
        );
        Ok((from_block, to_block))
    }

    async fn get_cancellations_between_blocks(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<OrderWithAuctionId>> {
        let mut order_updates = Vec::new();
        let events = self
            .contract
            .events()
            .cancellation_sell_order()
            .from_block(BlockNumber::Number(from_block.into()))
            .to_block(BlockNumber::Number(to_block.into()))
            .query()
            .await?;
        for event in events {
            let order = Order {
                sell_amount: U256::from(event.data.sell_amount),
                buy_amount: U256::from(event.data.buy_amount),
                user_id: event.data.user_id as u64,
            };
            let order_update = OrderWithAuctionId {
                auction_id: event.data.auction_id.as_u64(),
                order,
            };
            order_updates.push(order_update);
        }
        Ok(order_updates)
    }
}

fn get_address_from_bytes(input: Vec<u8>) -> ethcontract::H160 {
    if input.len() == 32 {
        return ethabi::decode(&[ParamType::Address], &input)
            .unwrap_or_else(|_| vec![ethabi::Token::Address(H160::zero())])
            .get(0)
            .unwrap()
            .clone()
            .into_address()
            .unwrap();
    } else if input.len() == 20 {
        let vec_as_array: [u8; 20] = input.try_into().unwrap_or_else(|v: Vec<u8>| {
            panic!("Expected a Vec of length {} but it was {}", 4, v.len())
        });
        return ethcontract::H160::from(vec_as_array);
    }
    ethcontract::H160::zero()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn abi_decode_bytes() {
        let vec_u8_short: Vec<u8> = hex!("740a98f8f4fae0986fb3264fe4aacf94ac1ee96f").to_vec();
        let vec_u8_long: Vec<u8> =
            hex!("000000000000000000000000740a98f8f4fae0986fb3264fe4aacf94ac1ee96f").to_vec();

        let address_from_long: Address = get_address_from_bytes(vec_u8_long);
        let address_from_short: Address = get_address_from_bytes(vec_u8_short);

        let original_address: H160 = "740a98f8f4fae0986fb3264fe4aacf94ac1ee96f".parse().unwrap();
        assert_eq!(address_from_long, original_address);
        assert_eq!(address_from_short, original_address);
    }
}
