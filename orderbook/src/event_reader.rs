use anyhow::Result;
use contracts::EasyAuction;
use ethcontract::BlockNumber;
use model::order::Order;
use model::user::User;
use primitive_types::U256;
use tracing::info;
use web3::Web3;

pub struct EventReader {
    pub contract: EasyAuction,
    pub web3: Web3<web3::transports::Http>,
}

pub struct OrderUpdates {
    pub orders_added: Vec<Order>,
    pub orders_removed: Vec<Order>,
    pub orders_claimed: Vec<Order>,
    pub users_added: Vec<User>,
    pub last_block_handled: u64,
}

const BLOCK_CONFIRMATION_COUNT: u64 = 10;

impl EventReader {
    pub fn new(contract: EasyAuction, web3: Web3<web3::transports::Http>) -> Self {
        Self { contract, web3 }
    }

    pub async fn get_order_updates(
        &self,
        last_handled_block: u64,
        auction_id: u64,
        reorg_protection: bool,
    ) -> Result<OrderUpdates> {
        let (from_block, to_block) = self
            .get_to_block(last_handled_block, auction_id, reorg_protection)
            .await?;
        let orders_added = self
            .get_order_placements_between_blocks(from_block, to_block, auction_id)
            .await?;
        let orders_removed = self
            .get_cancellations_between_blocks(from_block, to_block, auction_id)
            .await?;
        let orders_claimed = self
            .get_order_claims_between_blocks(from_block, to_block, auction_id)
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

    async fn get_order_placements_between_blocks(
        &self,
        from_block: u64,
        to_block: u64,
        auction_id: u64,
    ) -> Result<Vec<Order>> {
        let mut orders = Vec::new();
        let events = self
            .contract
            .events()
            .new_sell_order()
            .from_block(BlockNumber::Number(from_block.into()))
            .to_block(BlockNumber::Number(to_block.into()))
            .auction_id(U256::from(auction_id).into())
            .query()
            .await?;
        for event in events {
            let order = Order {
                sell_amount: U256::from(event.data.sell_amount),
                buy_amount: U256::from(event.data.buy_amount),
                user_id: event.data.user_id as u64,
            };
            orders.push(order);
        }
        Ok(orders)
    }
    async fn get_order_claims_between_blocks(
        &self,
        from_block: u64,
        to_block: u64,
        auction_id: u64,
    ) -> Result<Vec<Order>> {
        let mut orders = Vec::new();
        let events = self
            .contract
            .events()
            .claimed_from_order()
            .from_block(BlockNumber::Number(from_block.into()))
            .to_block(BlockNumber::Number(to_block.into()))
            .auction_id(U256::from(auction_id).into())
            .query()
            .await?;
        for event in events {
            let order = Order {
                sell_amount: U256::from(event.data.sell_amount),
                buy_amount: U256::from(event.data.buy_amount),
                user_id: event.data.user_id as u64,
            };
            orders.push(order);
        }
        Ok(orders)
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

    async fn get_to_block(
        &self,
        last_handled_block: u64,
        auction_id: u64,
        reorg_protection: bool,
    ) -> Result<(u64, u64)> {
        let current_block = self.web3.eth().block_number().await?.as_u64();
        let mut to_block = current_block;
        if reorg_protection {
            to_block -= BLOCK_CONFIRMATION_COUNT;
        }
        let from_block = last_handled_block + 1;
        if from_block > to_block {
            anyhow::bail!("Benign Error: from_block > to_block for updating events")
        }
        info!(
            "Updating event based orderbook from block {} to block {} for auctionId {}.",
            from_block, to_block, auction_id,
        );
        Ok((from_block, to_block))
    }

    async fn get_cancellations_between_blocks(
        &self,
        from_block: u64,
        to_block: u64,
        auction_id: u64,
    ) -> Result<Vec<Order>> {
        let mut orders = Vec::new();
        let events = self
            .contract
            .events()
            .cancellation_sell_order()
            .from_block(BlockNumber::Number(from_block.into()))
            .to_block(BlockNumber::Number(to_block.into()))
            .auction_id(U256::from(auction_id).into())
            .query()
            .await?;
        for event in events {
            let order = Order {
                sell_amount: U256::from(event.data.sell_amount),
                buy_amount: U256::from(event.data.buy_amount),
                user_id: event.data.user_id as u64,
            };
            orders.push(order);
        }
        Ok(orders)
    }
}
