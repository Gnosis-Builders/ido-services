use crate::event_reader::EventReader;
use anyhow::Result;
use lazy_static::lazy_static;
use model::order::Order;
use primitive_types::U256;
use std::collections::{hash_map::Entry, HashMap};
use tokio::sync::RwLock;

#[derive(Default, Debug)]
pub struct Orderbook {
    pub orders: RwLock<HashMap<u64, Vec<Order>>>,
}
lazy_static! {
    pub static ref QUEUE_START: Order = Order {
        buy_amount: U256::from("0"),
        sell_amount: U256::from("0"),
        user_id: 0 as u64,
    };
}
impl Orderbook {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Orderbook {
            orders: RwLock::new(HashMap::new()),
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
                        smaller_order = order_from_vec.clone();
                    }
                }
                smaller_order
            }
            Entry::Vacant(_) => *QUEUE_START,
        }
    }
    pub async fn run_maintenance(
        &self,
        event_reader: &EventReader,
        last_block_considered: u64,
        reorg_protection: bool,
    ) -> Result<u64> {
        let max_auction_id = event_reader.contract.auction_counter().call().await?;
        let mut last_considered_block = 0;
        for auction_id in 1..(max_auction_id.low_u64() + 1) {
            let (new_orders, last_considered_block_from_events) = event_reader
                .get_newly_placed_orders(last_block_considered, auction_id, reorg_protection)
                .await
                .expect("Could not get new orders");
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
        Ok(last_considered_block)
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
        orderbook.insert_order(auction_id, order.clone()).await;
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
        orderbook.insert_order(auction_id, order_1.clone()).await;
        let order_2 = Order {
            sell_amount: U256::from_dec_str("1230").unwrap(),
            buy_amount: U256::from_dec_str("12").unwrap(),
            user_id: 10 as u64,
        };
        orderbook.insert_order(auction_id, order_2.clone()).await;
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
        orderbook.insert_order(auction_id, order_1.clone()).await;
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
