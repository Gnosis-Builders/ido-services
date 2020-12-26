use contracts::EasyAuction;
use model::order::Order;
use std::collections::{hash_map::Entry, HashMap};

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Orderbook {
    orders: HashMap<u64, Vec<Order>>,
    last_block_considered: u64,
}

impl Orderbook {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Orderbook {
            orders: HashMap::new(),
            last_block_considered: 0,
        }
    }
    #[allow(dead_code)]
    pub fn insert_order(&mut self, auction_id: u64, order: Order) {
        match self.orders.entry(auction_id) {
            Entry::Occupied(mut order_vec) => {
                order_vec.get_mut().push(order);
            }
            Entry::Vacant(_) => {
                self.orders.insert(auction_id, vec![order]);
            }
        }
    }
    #[allow(dead_code)]
    pub fn sort_orders(&mut self, auction_id: u64) {
        match self.orders.entry(auction_id) {
            Entry::Occupied(mut order_vec) => {
                order_vec.get_mut().sort();
            }
            Entry::Vacant(_) => {}
        }
    }
    #[allow(dead_code)]
    fn remove_order(&mut self, auction_id: u64, order: Order) -> bool {
        match self.orders.entry(auction_id) {
            Entry::Occupied(order_vec) => {
                order_vec.into_mut().retain(|x| *x != order);
                true
            }
            Entry::Vacant(_) => false,
        }
    }
    pub async fn run_maintenance_with_reorg_protection(&self, _contract: &EasyAuction) {
        unimplemented!()
    }
    pub async fn run_maintenance(&self, _contract: &EasyAuction) {
        unimplemented!()
    }
}

mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use primitive_types::U256;

    #[test]
    fn adds_order_to_orderbook() {
        let order = Order {
            sell_amount: U256::from_dec_str("1230").unwrap(),
            buy_amount: U256::from_dec_str("123").unwrap(),
            user_id: 10 as u64,
        };
        let auction_id = 1;
        let mut orderbook = Orderbook::new();
        orderbook.insert_order(auction_id, order.clone());
        assert_eq!(
            *(*orderbook.orders.get(&auction_id).unwrap())
                .get(0)
                .unwrap(),
            order
        );
    }
}
