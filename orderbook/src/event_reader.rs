use anyhow::{ensure, Result};
use contracts::EasyAuction;
use ethcontract::BlockNumber;
use model::order::Order;
use primitive_types::U256;
use tracing::info;
use web3::Web3;

pub struct EventReader {
    pub contract: EasyAuction,
    web3: Web3<web3::transports::Http>,
}

const BLOCK_CONFIRMATION_COUNT: u64 = 10;

impl EventReader {
    pub fn new(contract: EasyAuction, web3: Web3<web3::transports::Http>) -> Self {
        Self { contract, web3 }
    }
    /// Gather all new events since the last update and update the orderbook.
    pub async fn get_newly_placed_orders(
        &self,
        last_handled_block: u64,
        auction_id: u64,
        reorg_protection: bool,
    ) -> Result<(Vec<Order>, u64)> {
        let current_block = self.web3.eth().block_number().await?.as_u64();
        let mut to_block = current_block;
        if reorg_protection {
            to_block -= BLOCK_CONFIRMATION_COUNT;
        }
        let from_block = last_handled_block.clone();
        ensure!(
            from_block <= to_block,
            format!(
                "current block number according to node is {} which is more than {} blocks in the \
             past compared to previous current block {}",
                to_block, BLOCK_CONFIRMATION_COUNT, from_block
            )
        );
        info!(
            "Updating event based orderbook from block {} to block {} for auctionId {}.",
            from_block, to_block, auction_id,
        );
        let orders = self
            .update_with_events_between_blocks(last_handled_block, to_block, auction_id)
            .await
            .expect("Orders could not be downloaded");
        Ok((orders, to_block))
    }

    async fn update_with_events_between_blocks(
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
}
