use crate::event_reader::EventReader;
use anyhow::Result;
use model::order::Order;
use std::collections::{hash_map::Entry, HashMap};
use tokio::sync::RwLock;

#[derive(Default, Debug)]
pub struct Orderbook {
    pub orders: RwLock<HashMap<u64, Vec<Order>>>,
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
        let orders = orderbook.orders.read().await;
        assert_eq!(*(*orders.get(&auction_id).unwrap()).get(0).unwrap(), order);
    }
}
