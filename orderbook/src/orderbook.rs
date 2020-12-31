use crate::event_reader::EventReader;
use anyhow::Result;
use ethcontract::Address;
use hex::encode;
use lazy_static::lazy_static;
use model::order::{Order, OrderbookDisplay, PricePoint};
use primitive_types::U256;
use std::collections::{hash_map::Entry, HashMap};
use std::str::FromStr;
use tokio::sync::RwLock;

#[derive(Default, Debug)]
pub struct Orderbook {
    pub orders: RwLock<HashMap<u64, Vec<Order>>>,
    pub initial_order: RwLock<HashMap<u64, Order>>,
    pub decimals_auctioning_token: RwLock<Option<U256>>,
    pub decimals_bidding_token: RwLock<Option<U256>>,
}
lazy_static! {
    pub static ref QUEUE_START: Order = Order {
        buy_amount: U256::from_dec_str("0").unwrap(),
        sell_amount: U256::from_dec_str("1").unwrap(),
        user_id: 0 as u64,
    };
    pub static ref EIGHTEEN: U256 = U256::from_dec_str("18").unwrap();
}
impl Orderbook {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Orderbook {
            orders: RwLock::new(HashMap::new()),
            initial_order: RwLock::new(HashMap::new()),
            decimals_auctioning_token: RwLock::new(None),
            decimals_bidding_token: RwLock::new(None),
        }
    }
    #[allow(dead_code)]
    pub async fn insert_order(&mut self, auction_id: u64, order: Order) {
        let mut hashmap = self.orders.write().await;
        match hashmap.entry(auction_id) {
            Entry::Occupied(mut order_vec) => {
                order_vec.get_mut().push(order);
            }
            Entry::Vacant(_) => {
                hashmap.insert(auction_id, vec![order]);
            }
        }
    }
    pub async fn get_initial_order(&self, auction_id: u64) -> Order {
        let order_hashmap = self.initial_order.read().await;
        if let Some(order) = order_hashmap.get(&auction_id) {
            *order
        } else {
            *QUEUE_START
        }
    }
    pub async fn update_initial_order(&mut self, auction_id: u64, order: Order) {
        let mut order_hashmap = self.initial_order.write().await;
        order_hashmap.insert(auction_id, order);
    }
    pub async fn is_initial_order_set(&self, auction_id: u64) -> bool {
        let order_hashmap = self.initial_order.read().await;
        order_hashmap.contains_key(&auction_id)
    }
    #[allow(dead_code)]
    pub async fn sort_orders(&mut self, auction_id: u64) {
        let mut hashmap = self.orders.write().await;
        match hashmap.entry(auction_id) {
            Entry::Occupied(mut order_vec) => {
                order_vec.get_mut().sort();
            }
            Entry::Vacant(_) => {}
        }
    }
    #[allow(dead_code)]
    pub async fn remove_order(&mut self, auction_id: u64, order: Order) -> bool {
        let mut hashmap = self.orders.write().await;
        match hashmap.entry(auction_id) {
            Entry::Occupied(order_vec) => {
                order_vec.into_mut().retain(|x| *x != order);
                true
            }
            Entry::Vacant(_) => false,
        }
    }
    pub async fn get_order_book_display(&self, auction_id: u64) -> Result<OrderbookDisplay> {
        let orders_hashmap = self.orders.write().await;
        let decimals_auctioning_token = self
            .decimals_auctioning_token
            .read()
            .await
            .expect("auction not yet initialized in backend");
        let decimals_bidding_token = self
            .decimals_bidding_token
            .read()
            .await
            .expect("auction not yet initialized in backend");
        let bids: Vec<PricePoint>;
        if let Some(orders) = orders_hashmap.get(&auction_id) {
            bids = orders
                .iter()
                .map(|order| {
                    order.to_price_point(decimals_auctioning_token, decimals_bidding_token)
                })
                .collect();
        } else {
            bids = Vec::new();
        }
        let initial_order = vec![self.get_initial_order(auction_id).await];
        let asks: Vec<PricePoint> = initial_order
            .iter()
            .map(|order| Order {
                // << invert price for unified representation of different orders.
                sell_amount: order.buy_amount,
                buy_amount: order.sell_amount,
                user_id: order.user_id,
            })
            .map(|order| order.to_price_point(decimals_auctioning_token, decimals_bidding_token))
            .collect();
        Ok(OrderbookDisplay { asks, bids })
    }
    #[allow(dead_code)]
    pub async fn get_orders(&mut self, auction_id: u64) -> Vec<Order> {
        let mut hashmap = self.orders.write().await;
        match hashmap.entry(auction_id) {
            Entry::Occupied(order_vec) => order_vec.get().clone(),
            Entry::Vacant(_) => Vec::new(),
        }
    }
    pub async fn get_previous_order(&self, auction_id: u64, order: Order) -> Order {
        let mut order_hashmap = self.orders.write().await;
        match order_hashmap.entry(auction_id) {
            Entry::Occupied(order_vec) => {
                let mut smaller_order: Order = *QUEUE_START;
                for order_from_vec in order_vec.get() {
                    if order_from_vec < &order {
                        smaller_order = *order_from_vec;
                    }
                }
                smaller_order
            }
            Entry::Vacant(_) => *QUEUE_START,
        }
    }
    pub async fn initial_setup_if_not_yet_done(
        &self,
        auction_id: u64,
        event_reader: &EventReader,
    ) -> Result<()> {
        if !self.is_initial_order_set(auction_id).await {
            let auction_data = event_reader
                .contract
                .auction_data(U256::from(auction_id))
                .call()
                .await?;
            let auctioning_token: Address = Address::from(auction_data.0);
            let bidding_token: Address = Address::from(auction_data.1);
            let initial_order: Order = FromStr::from_str(&encode(&auction_data.3))?;
            self.set_decimals_for_auctioning_token(event_reader, auctioning_token)
                .await?;
            self.set_decimals_for_bidding_token(event_reader, bidding_token)
                .await?;
            let mut order_hashmap = self.initial_order.write().await;
            order_hashmap.insert(auction_id, initial_order);
        }
        Ok(())
    }
    pub async fn set_decimals_for_auctioning_token(
        &self,
        event_reader: &EventReader,
        token_address: Address,
    ) -> Result<()> {
        let erc20_contract = contracts::ERC20::at(&event_reader.web3, token_address);
        let mut decimals = self.decimals_auctioning_token.write().await;
        *decimals = Some(U256::from(erc20_contract.decimals().call().await?));
        Ok(())
    }
    pub async fn set_decimals_for_bidding_token(
        &self,
        event_reader: &EventReader,
        token_address: Address,
    ) -> Result<()> {
        let erc20_contract = contracts::ERC20::at(&event_reader.web3, token_address);
        let mut decimals = self.decimals_bidding_token.write().await;
        *decimals = Some(U256::from(erc20_contract.decimals().call().await?));
        Ok(())
    }
    pub async fn run_maintenance(
        &self,
        event_reader: &EventReader,
        last_block_considered: u64,
        reorg_protection: bool,
    ) -> Result<u64> {
        let max_auction_id = event_reader
            .contract
            .auction_counter()
            .call()
            .await
            .unwrap_or(U256::zero());
        let mut last_considered_block = 0;
        for auction_id in 1..=(max_auction_id.low_u64()) {
            match self
                .initial_setup_if_not_yet_done(auction_id, event_reader)
                .await
            {
                Err(err) => {
                    tracing::info!(
                        "update_initial_order_if_not_set was not successful for auction_id {:?} with error: {:}",
                        auction_id,
                        err
                    );
                    break;
                }
                _ => (),
            };
            let new_orders: Vec<Order>;
            let last_considered_block_from_events: u64;
            match event_reader
                .get_newly_placed_orders(last_block_considered, auction_id, reorg_protection)
                .await
            {
                Ok((orders, last_considered_block)) => {
                    new_orders = orders;
                    last_considered_block_from_events = last_considered_block;
                }
                Err(err) => {
                    tracing::info!(
                        "get_newly_placed_orders was not successful for auction_id {:?} with error: {:}",
                        auction_id,
                        err
                    );
                    break;
                }
            }
            last_considered_block =
                std::cmp::min(last_considered_block_from_events, last_considered_block);
            if last_considered_block == 0 {
                last_considered_block = last_considered_block_from_events;
            }
            let mut orders = self.orders.write().await;
            let entry = orders.entry(auction_id);
            match entry {
                Entry::Occupied(orders) => orders.into_mut().extend(new_orders),
                Entry::Vacant(empty_vec) => {
                    empty_vec.insert(new_orders);
                }
            }
        }
        Ok(last_considered_block + 1)
    }
}

mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use primitive_types::U256;

    #[tokio::test]
    async fn adds_order_to_orderbook() {
        let order = Order {
            sell_amount: U256::from_dec_str("1230").unwrap(),
            buy_amount: U256::from_dec_str("123").unwrap(),
            user_id: 10 as u64,
        };
        let auction_id = 1;
        let mut orderbook = Orderbook::new();
        orderbook.insert_order(auction_id, order).await;
        assert_eq!(orderbook.get_orders(auction_id).await, vec![order]);
    }

    #[tokio::test]
    async fn sorts_orders_from_orderbook() {
        let order_1 = Order {
            sell_amount: U256::from_dec_str("1230").unwrap(),
            buy_amount: U256::from_dec_str("123").unwrap(),
            user_id: 10 as u64,
        };
        let auction_id = 1;
        let mut orderbook = Orderbook::new();
        orderbook.insert_order(auction_id, order_1).await;
        let order_2 = Order {
            sell_amount: U256::from_dec_str("1230").unwrap(),
            buy_amount: U256::from_dec_str("12").unwrap(),
            user_id: 10 as u64,
        };
        orderbook.insert_order(auction_id, order_2).await;
        orderbook.sort_orders(auction_id).await;
        assert_eq!(
            orderbook.get_orders(auction_id).await,
            vec![order_2, order_1]
        );
    }

    #[tokio::test]
    async fn get_previous_order() {
        let order_1 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 10 as u64,
        };
        let auction_id = 1;
        let mut orderbook = Orderbook::new();
        assert_eq!(
            orderbook.get_previous_order(auction_id, order_1).await,
            *QUEUE_START
        );
        orderbook.insert_order(auction_id, order_1).await;
        let order_2 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("3").unwrap(),
            user_id: 10 as u64,
        };
        assert_eq!(
            orderbook.get_previous_order(auction_id, order_2).await,
            order_1
        );
    }
}
