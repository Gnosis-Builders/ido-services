use crate::event_reader::EventReader;
use crate::subgraph::uniswap_graph_api::UniswapSubgraphClient;
use anyhow::{anyhow, Result};
use ethcontract::Address;
use ethcontract::H160;
use lazy_static::lazy_static;
use maplit::hashmap;
use model::auction_details::AuctionDetails;
use model::order::TEN;
use model::order::{Order, OrderWithAuctionId, OrderbookDisplay, PricePoint};
use model::user::User;
use primitive_types::U256;
use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::str::FromStr;
use std::time::SystemTime;
use tokio::sync::RwLock;

#[derive(Default, Debug)]
pub struct Orderbook {
    pub orders: RwLock<HashMap<u64, Vec<Order>>>,
    pub orders_display: RwLock<HashMap<u64, Vec<PricePoint>>>,
    pub orders_without_claimed: RwLock<HashMap<u64, Vec<Order>>>,
    pub users: RwLock<HashMap<Address, u64>>,
    pub auction_participation: RwLock<HashMap<u64, HashSet<u64>>>,
    pub auction_details: RwLock<HashMap<u64, AuctionDetails>>,
}
lazy_static! {
    pub static ref LEGIT_STABLE_COINS: HashMap::<u32, Vec<Address>> = hashmap! {
        1 => vec![Address::from_str("0x6b175474e89094c44da98b954eedeac495271d0f").unwrap(),Address::from_str("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap(), Address::from_str("0xdac17f958d2ee523a2206206994597c13d831ec7").unwrap()],
        4 => vec![Address::from_str("0x5592EC0cfb4dbc12D3aB100b257153436a1f0FEa").unwrap(),Address::from_str("0x4DBCdF9B62e891a7cec5A2568C3F4FAF9E8Abe2b").unwrap()],
        100 => vec![Address::from_str("0xe91D153E0b41518A2Ce8Dd3D7944Fa863463a97d").unwrap()],
        137 => vec![Address::from_str("0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063").unwrap(), Address::from_str("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174").unwrap()],
    };
    pub static ref PRICE_FEED_SUPPORTED_TOKENS: HashMap::<u32, Vec<Address>> = hashmap! {
     1 => vec![Address::from_str("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2").unwrap()],
    };
}

lazy_static! {
    pub static ref QUEUE_START: Order = Order {
        buy_amount: U256::from_dec_str("0").unwrap(),
        sell_amount: U256::from_dec_str("1").unwrap(),
        user_id: 0_u64,
    };
}
impl Orderbook {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Orderbook {
            orders: RwLock::new(HashMap::new()),
            orders_display: RwLock::new(HashMap::new()),
            orders_without_claimed: RwLock::new(HashMap::new()),
            users: RwLock::new(HashMap::new()),
            auction_participation: RwLock::new(HashMap::new()),
            auction_details: RwLock::new(HashMap::new()),
        }
    }
    pub async fn insert_orders(&self, auction_id: u64, orders: Vec<Order>) {
        if orders.is_empty() {
            return;
        }
        {
            let mut hashmap = self.orders.write().await;
            match hashmap.entry(auction_id) {
                Entry::Occupied(mut order_vec) => {
                    order_vec.get_mut().extend(orders.clone());
                }
                Entry::Vacant(_) => {
                    hashmap.insert(auction_id, orders.clone());
                }
            }
        }
        {
            let mut hashmap = self.orders_without_claimed.write().await;
            match hashmap.entry(auction_id) {
                Entry::Occupied(mut order_vec) => {
                    order_vec.get_mut().extend(orders.clone());
                }
                Entry::Vacant(_) => {
                    hashmap.insert(auction_id, orders.clone());
                }
            }
        }
        let (decimals_auctioning_token, decimals_bidding_token) =
            self.get_decimals(auction_id).await;
        {
            let mut hashmap = self.orders_display.write().await;
            let vec_price_points: Vec<PricePoint> = orders
                .iter()
                .map(|order| {
                    order.convert_to_price_point(decimals_auctioning_token, decimals_bidding_token)
                })
                .collect();
            match hashmap.entry(auction_id) {
                Entry::Occupied(mut order_vec) => {
                    order_vec.get_mut().extend(vec_price_points);
                }
                Entry::Vacant(_) => {
                    hashmap.insert(auction_id, vec_price_points);
                }
            }
        }
        {
            let vec_users: HashSet<u64> = orders
                .into_iter()
                .map(|order| order.user_id)
                .collect::<HashSet<u64>>()
                .into_iter()
                .collect();
            for user_id in vec_users {
                let mut hashmap = self.auction_participation.write().await;
                match hashmap.entry(user_id) {
                    Entry::Occupied(mut auction_set) => {
                        auction_set.get_mut().insert(auction_id);
                    }
                    Entry::Vacant(_) => {
                        let mut new_hash_set = HashSet::new();
                        new_hash_set.insert(auction_id);
                        hashmap.insert(user_id, new_hash_set.clone());
                    }
                }
            }
        }
    }
    pub async fn insert_users(&self, users: Vec<User>) {
        if users.is_empty() {
            return;
        }
        let mut hashmap = self.users.write().await;
        for user in users {
            hashmap.insert(user.address, user.user_id);
        }
    }
    pub async fn update_initial_order(&mut self, auction_id: u64, order: Order) {
        let mut hashmap = self.auction_details.write().await;
        match hashmap.entry(auction_id) {
            Entry::Occupied(mut auction_details) => {
                auction_details.get_mut().exact_order = order;
            }
            Entry::Vacant(_) => {
                hashmap.insert(
                    auction_id,
                    AuctionDetails {
                        auction_id,
                        exact_order: order,
                        ..Default::default()
                    },
                );
            }
        }
    }
    pub async fn sort_orders(&self, auction_id: u64) {
        let mut hashmap = self.orders.write().await;
        match hashmap.entry(auction_id) {
            Entry::Occupied(order_vec) => {
                order_vec.into_mut().sort();
            }
            Entry::Vacant(_) => {}
        }
    }
    pub async fn sort_orders_display(&self, auction_id: u64) {
        let mut hashmap = self.orders_display.write().await;
        match hashmap.entry(auction_id) {
            Entry::Occupied(order_vec) => {
                order_vec.into_mut().sort();
            }
            Entry::Vacant(_) => {}
        }
    }
    pub async fn sort_orders_without_claimed(&self, auction_id: u64) {
        let mut hashmap = self.orders_without_claimed.write().await;
        match hashmap.entry(auction_id) {
            Entry::Occupied(order_vec) => {
                order_vec.into_mut().sort();
            }
            Entry::Vacant(_) => {}
        }
    }
    pub async fn get_decimals(&self, auction_id: u64) -> (U256, U256) {
        let decimals_auctioning_token;
        let decimals_bidding_token;
        {
            let reading_guard = self.auction_details.read().await;
            decimals_auctioning_token = reading_guard
                .get(&auction_id)
                .expect("auction not yet initialized in backend")
                .decimals_auctioning_token;

            decimals_bidding_token = reading_guard
                .get(&auction_id)
                .expect("auction not yet initialized in backend")
                .decimals_bidding_token;
        }
        (decimals_auctioning_token, decimals_bidding_token)
    }
    pub async fn remove_orders(&self, auction_id: u64, orders: Vec<Order>) {
        if orders.is_empty() {
            return;
        }
        {
            let mut hashmap = self.orders.write().await;
            match hashmap.entry(auction_id) {
                Entry::Occupied(order_vec) => {
                    order_vec.into_mut().retain(|x| !orders.contains(x));
                }
                Entry::Vacant(_) => (),
            }
        }
        let (decimals_auctioning_token, decimals_bidding_token) =
            self.get_decimals(auction_id).await;
        {
            let mut hashmap = self.orders_display.write().await;
            let vec_price_points: Vec<PricePoint> = orders
                .iter()
                .map(|order| {
                    order.convert_to_price_point(decimals_auctioning_token, decimals_bidding_token)
                })
                .collect();
            match hashmap.entry(auction_id) {
                Entry::Occupied(order_vec) => {
                    order_vec
                        .into_mut()
                        .retain(|x| !vec_price_points.contains(x));
                }
                Entry::Vacant(_) => (),
            }
        }
        {
            let mut hashmap = self.orders_without_claimed.write().await;
            match hashmap.entry(auction_id) {
                Entry::Occupied(order_vec) => {
                    order_vec.into_mut().retain(|x| !orders.contains(x));
                }
                Entry::Vacant(_) => (),
            }
        }
    }
    pub async fn remove_claimed_orders(&self, auction_id: u64, orders: Vec<Order>) -> bool {
        if orders.is_empty() {
            return true;
        }
        let mut hashmap = self.orders_without_claimed.write().await;
        match hashmap.entry(auction_id) {
            Entry::Occupied(order_vec) => {
                order_vec.into_mut().retain(|x| !orders.contains(x));
                true
            }
            Entry::Vacant(_) => false,
        }
    }
    pub async fn get_initial_order(&self, auction_id: u64) -> Order {
        let auction_details_hashmap = self.auction_details.read().await;
        if let Some(auction_details) = auction_details_hashmap.get(&auction_id) {
            auction_details.exact_order
        } else {
            *QUEUE_START
        }
    }
    pub async fn get_order_book_display(&self, auction_id: u64) -> Result<OrderbookDisplay> {
        let bids: Vec<PricePoint>;
        let (decimals_auctioning_token, decimals_bidding_token) =
            self.get_decimals(auction_id).await;
        {
            let orders_hashmap = self.orders_display.read().await;
            if let Some(orders) = orders_hashmap.get(&auction_id) {
                bids = orders.to_vec();
            } else {
                bids = Vec::new();
            }
        }
        let initial_order = vec![self.get_initial_order(auction_id).await];
        let asks: Vec<PricePoint> = initial_order
            .iter()
            .map(|order| Order {
                sell_amount: order.sell_amount,
                buy_amount: order.buy_amount,
                user_id: order.user_id,
            })
            .map(|order| {
                order
                    .convert_to_price_point(decimals_bidding_token, decimals_auctioning_token)
                    // << invert price for unified representation of different orders.
                    .invert_price()
            })
            .collect();
        Ok(OrderbookDisplay { asks, bids })
    }
    pub async fn get_orders(&self, auction_id: u64) -> Vec<Order> {
        let hashmap = self.orders.read().await;
        let empty_vec = Vec::new();
        hashmap.get(&auction_id).unwrap_or(&empty_vec).clone()
    }
    pub async fn get_used_auctions(&self, user_id: u64) -> HashSet<u64> {
        let hashmap = self.auction_participation.read().await;
        let empty_set = HashSet::new();
        let result = hashmap.get(&user_id).unwrap_or(&empty_set).clone();
        result
    }
    pub async fn get_user_id(&self, user: H160) -> Result<u64> {
        let hashmap = self.users.read().await;
        Ok(*hashmap.get(&user).unwrap_or(&(0_u64)))
    }

    pub async fn get_user_orders(&self, auction_id: u64, user: H160) -> Vec<Order> {
        let hashmap = self.users.read().await;
        let user_id = *hashmap.get(&user).unwrap_or(&(0_u64));
        let hashmap = self.orders.read().await;
        let empty_vec = Vec::new();
        let current_orders = hashmap.get(&auction_id).unwrap_or(&empty_vec);
        current_orders
            .iter()
            .filter(|order| order.user_id == user_id)
            .copied()
            .collect()
    }
    pub async fn get_user_orders_without_canceled_claimed(
        &self,
        auction_id: u64,
        user: H160,
    ) -> Vec<Order> {
        let hashmap = self.users.read().await;
        let user_id = *hashmap.get(&user).unwrap_or(&(0_u64));
        let hashmap = self.orders_without_claimed.read().await;
        let empty_vec = Vec::new();
        let current_orders = hashmap.get(&auction_id).unwrap_or(&empty_vec);
        current_orders
            .iter()
            .filter(|order| order.user_id == user_id)
            .copied()
            .collect()
    }
    pub async fn get_clearing_order_and_volume(
        &self,
        auction_id: u64,
    ) -> Result<(Order, U256, U256)> {
        // code is one to one copy of smart contract, hence no extensive testing
        let orders = self.get_orders(auction_id).await;
        let initial_order = self.get_initial_order(auction_id).await;
        let mut current_bid_sum = U256::zero();
        let mut current_order = Order::default();

        for order in orders {
            current_order = order;
            current_bid_sum = current_bid_sum
                .checked_add(order.sell_amount)
                .ok_or_else(|| anyhow!("error in get_clearing_price_calculation"))?;
            if current_bid_sum
                .checked_mul(order.buy_amount)
                .ok_or_else(|| anyhow!("error in get_clearing_price_calculation"))?
                .ge(&initial_order
                    .sell_amount
                    .checked_mul(order.sell_amount)
                    .ok_or_else(|| anyhow!("error in get_clearing_price_calculation"))?)
            {
                break;
            }
        }
        if current_bid_sum.gt(&U256::zero())
            && current_bid_sum
                .checked_mul(current_order.buy_amount)
                .ok_or_else(|| anyhow!("error in get_clearing_price_calculation"))?
                .ge(&initial_order
                    .sell_amount
                    .checked_mul(current_order.sell_amount)
                    .ok_or_else(|| anyhow!("error in get_clearing_price_calculation"))?)
        {
            let uncovered_bids = current_bid_sum
                .checked_sub(
                    initial_order
                        .sell_amount
                        .checked_mul(current_order.sell_amount)
                        .ok_or_else(|| anyhow!("error in get_clearing_price_calculation"))?
                        .checked_div(current_order.buy_amount)
                        .ok_or_else(|| anyhow!("error in get_clearing_price_calculation"))?,
                )
                .ok_or_else(|| anyhow!("error in get_clearing_price_calculation"))?;
            if current_order.sell_amount.ge(&uncovered_bids) {
                let sell_amount_clearing_order = current_order
                    .sell_amount
                    .checked_sub(uncovered_bids)
                    .ok_or_else(|| anyhow!("error in get_clearing_price_calculation"))?;
                Ok((
                    current_order,
                    sell_amount_clearing_order,
                    current_bid_sum
                        .checked_sub(current_order.sell_amount)
                        .ok_or_else(|| anyhow!("error in get_clearing_price_calculation"))?
                        .checked_add(sell_amount_clearing_order)
                        .ok_or_else(|| anyhow!("error in get_clearing_price_calculation"))?,
                ))
            } else {
                let clearing_order = Order {
                    sell_amount: current_bid_sum
                        .checked_sub(current_order.sell_amount)
                        .unwrap(),
                    buy_amount: initial_order.sell_amount,
                    user_id: 0_u64,
                };
                Ok((
                    clearing_order,
                    U256::zero(),
                    current_bid_sum
                        .checked_sub(current_order.sell_amount)
                        .ok_or_else(|| anyhow!("error in get_clearing_price_calculation"))?,
                ))
            }
        } else if current_bid_sum.gt(&initial_order.buy_amount) {
            let clearing_order = Order {
                buy_amount: initial_order.sell_amount,
                sell_amount: current_bid_sum,
                user_id: 0_u64,
            };
            Ok((clearing_order, U256::zero(), current_bid_sum))
        } else {
            let clearing_order = Order {
                buy_amount: initial_order.sell_amount,
                sell_amount: initial_order.buy_amount,
                user_id: 0_u64,
            };
            let clearing_volume = current_bid_sum
                .checked_mul(initial_order.sell_amount)
                .ok_or_else(|| anyhow!("error in get_clearing_price_calculation"))?
                .checked_div(initial_order.buy_amount)
                .ok_or_else(|| anyhow!("error in get_clearing_price_calculation"))?;
            Ok((clearing_order, clearing_volume, current_bid_sum))
        }
    }
    pub async fn get_previous_order(&self, auction_id: u64, order: Order) -> Order {
        let order_hashmap = self.orders_without_claimed.read().await;
        let empty_order_vec = Vec::new();
        let order_vec = order_hashmap.get(&auction_id).unwrap_or(&empty_order_vec);
        let mut smaller_order: Order = *QUEUE_START;
        for order_from_vec in order_vec {
            if order_from_vec < &order && order_from_vec > &smaller_order {
                smaller_order = *order_from_vec;
            }
        }
        smaller_order
    }
    pub async fn set_auction_details(
        &self,
        auction_id: u64,
        details: AuctionDetails,
    ) -> Result<()> {
        let mut auction_details = self.auction_details.write().await;
        auction_details.insert(auction_id, details);
        Ok(())
    }
    pub async fn get_max_auction_id(&self) -> Result<u64> {
        let auction_details = self.auction_details.read().await;
        let max_auction_id = auction_details.keys().max().unwrap_or(&0_u64);
        Ok(*max_auction_id)
    }
    pub async fn run_maintenance(
        &self,
        event_reader: &EventReader,
        mut the_graph_reader: &mut UniswapSubgraphClient,
        last_block_considered: &mut u64,
        reorg_protection: bool,
        chain_id: u32,
        current_block: u64,
    ) -> Result<()> {
        let (from_block, to_block);
        match event_reader.get_to_block(*last_block_considered, reorg_protection, current_block) {
            Ok(return_data) => {
                from_block = return_data.0;
                to_block = return_data.1
            }
            Err(err) => {
                tracing::debug!(
                    "get_to_block was not successful with interruption: {:}",
                    err
                );
                return Ok(());
            }
        }

        let new_auctions: Vec<AuctionDetails>;
        match event_reader
            .get_auction_updates(from_block, to_block, chain_id)
            .await
        {
            Ok(auction_updates) => {
                new_auctions = auction_updates;
            }
            Err(err) => {
                tracing::info!("get_order_updates was not successful with error: {:}", err);
                return Ok(());
            }
        }
        for auction_details in new_auctions {
            self.set_auction_details(auction_details.auction_id, auction_details)
                .await?;
        }
        let new_orders: Vec<OrderWithAuctionId>;
        let canceled_orders: Vec<OrderWithAuctionId>;
        let new_claimed_orders: Vec<OrderWithAuctionId>;
        let new_users: Vec<User>;
        match event_reader.get_order_updates(from_block, to_block).await {
            Ok(order_updates) => {
                new_orders = order_updates.orders_added;
                canceled_orders = order_updates.orders_removed;
                new_claimed_orders = order_updates.orders_claimed;
                new_users = order_updates.users_added;
            }
            Err(err) => {
                tracing::info!("get_order_updates was not successful with error: {:}", err);
                return Ok(());
            }
        }
        self.insert_users(new_users).await;

        let max_auction_id = self.get_max_auction_id().await?;
        for auction_id in 1..=max_auction_id {
            self.insert_orders(
                auction_id,
                new_orders
                    .iter()
                    .filter(|order_with_auction_id| order_with_auction_id.auction_id == auction_id)
                    .map(|order_with_auction_id| order_with_auction_id.order)
                    .collect(),
            )
            .await;
            self.remove_orders(
                auction_id,
                canceled_orders
                    .iter()
                    .filter(|order_with_auction_id| order_with_auction_id.auction_id == auction_id)
                    .map(|order_with_auction_id| order_with_auction_id.order)
                    .collect(),
            )
            .await;
            self.remove_claimed_orders(
                auction_id,
                new_claimed_orders
                    .iter()
                    .filter(|order_with_auction_id| order_with_auction_id.auction_id == auction_id)
                    .map(|order_with_auction_id| order_with_auction_id.order)
                    .collect(),
            )
            .await;
            self.sort_orders_without_claimed(auction_id).await;
            self.sort_orders(auction_id).await;
            self.sort_orders_display(auction_id).await;
            if let Err(err) = self
                .update_clearing_price_info(&mut the_graph_reader, auction_id, chain_id)
                .await
            {
                tracing::debug!(
                    "error while calculating the clearing price: {:} for auction id {:}",
                    err,
                    auction_id
                )
            };
        }
        *last_block_considered = to_block;
        Ok(())
    }
    pub async fn update_clearing_price_info(
        &self,
        mut the_graph_reader: &mut UniswapSubgraphClient,
        auction_id: u64,
        chain_id: u32,
    ) -> Result<()> {
        let new_clearing_price = self.get_clearing_order_and_volume(auction_id).await?;
        let decimals_auctioning_token;
        let decimals_bidding_token;
        {
            let reading_guard = self.auction_details.read().await;
            decimals_auctioning_token = reading_guard
                .get(&auction_id)
                .expect("auction not yet initialized in backend")
                .decimals_auctioning_token;

            decimals_bidding_token = reading_guard
                .get(&auction_id)
                .expect("auction not yet initialized in backend")
                .decimals_bidding_token;
        }
        self.update_current_price_of_details(
            auction_id,
            new_clearing_price
                .0
                .convert_to_price_point(decimals_auctioning_token, decimals_bidding_token)
                .price,
        )
        .await?;
        self.update_current_bidding_amount_of_details(auction_id, new_clearing_price.2)
            .await?;
        self.update_interest_score(auction_id).await?;
        self.update_usd_amount_traded_of_details(&mut the_graph_reader, auction_id, chain_id)
            .await?;
        Ok(())
    }
    pub async fn get_most_interesting_auctions(
        &self,
        number_of_auctions: u64,
    ) -> Result<Vec<AuctionDetails>> {
        let auction_details_hashmap = self.auction_details.read().await;
        let mut non_closed_auctions: Vec<AuctionDetails> = Vec::new();
        for auction_id in auction_details_hashmap.keys() {
            let auction_details = auction_details_hashmap.get(auction_id).unwrap();
            if auction_details.end_time_timestamp
                > SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            {
                non_closed_auctions.push(auction_details.clone());
            }
        }
        non_closed_auctions.sort();
        non_closed_auctions.reverse();
        if non_closed_auctions.len() > number_of_auctions as usize {
            non_closed_auctions = non_closed_auctions[0..(number_of_auctions as usize)].to_vec()
        }
        Ok(non_closed_auctions)
    }
    pub async fn get_most_interesting_closed_auctions(
        &self,
        number_of_auctions: u64,
    ) -> Result<Vec<AuctionDetails>> {
        let auction_details_hashmap = self.auction_details.read().await;
        let mut closed_auctions: Vec<AuctionDetails> = Vec::new();
        for auction_id in auction_details_hashmap.keys() {
            let auction_details = auction_details_hashmap.get(auction_id).unwrap();
            if auction_details.end_time_timestamp
                < SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            {
                closed_auctions.push(auction_details.clone());
            }
        }
        closed_auctions.sort_by(|a, b| {
            a.usd_amount_traded
                .partial_cmp(&b.usd_amount_traded)
                .unwrap()
        });
        closed_auctions.reverse();
        if closed_auctions.len() > number_of_auctions as usize {
            closed_auctions = closed_auctions[0..(number_of_auctions as usize)].to_vec()
        }
        Ok(closed_auctions)
    }
    pub async fn get_all_auction_with_details(&self) -> Result<Vec<AuctionDetails>> {
        let auction_details_hashmap = self.auction_details.read().await;
        let mut auction_detail_list: Vec<AuctionDetails> = Vec::new();
        for auction_id in auction_details_hashmap.keys() {
            let auction_details = auction_details_hashmap.get(auction_id).unwrap();
            auction_detail_list.push(auction_details.clone());
        }
        Ok(auction_detail_list)
    }
    pub async fn get_auction_with_details(&self, auction_id: u64) -> Result<AuctionDetails> {
        let auction_details_hashmap = self.auction_details.read().await;
        Ok(auction_details_hashmap.get(&auction_id).unwrap().clone())
    }
    pub async fn update_current_price_of_details(&self, auction_id: u64, price: f64) -> Result<()> {
        let mut auction_details_hashmap = self.auction_details.write().await;
        match auction_details_hashmap.entry(auction_id) {
            Entry::Occupied(mut details) => {
                details.get_mut().current_clearing_price = price;
            }
            Entry::Vacant(_) => {}
        }
        Ok(())
    }
    pub async fn update_current_bidding_amount_of_details(
        &self,
        auction_id: u64,
        amount: U256,
    ) -> Result<()> {
        let mut auction_details_hashmap = self.auction_details.write().await;
        match auction_details_hashmap.entry(auction_id) {
            Entry::Occupied(mut details) => {
                details.get_mut().current_bidding_amount = amount;
            }
            Entry::Vacant(_) => {}
        }
        Ok(())
    }
    pub async fn update_interest_score(&self, auction_id: u64) -> Result<()> {
        let mut auction_details_hashmap = self.auction_details.write().await;
        match auction_details_hashmap.entry(auction_id) {
            Entry::Occupied(mut details) => {
                details.get_mut().interest_score = details.get().current_bidding_amount.as_u128()
                    as f64
                    / (TEN.pow(details.get().decimals_bidding_token)).as_u128() as f64;
            }
            Entry::Vacant(_) => {}
        }
        Ok(())
    }
    pub async fn update_usd_amount_traded_of_details(
        &self,
        the_graph_reader: &mut UniswapSubgraphClient,
        auction_id: u64,
        chain_id: u32,
    ) -> Result<()> {
        let usd_amount;
        {
            let auction_details_hashmap = self.auction_details.read().await;
            usd_amount = match auction_details_hashmap.get(&auction_id) {
                Some(details) => {
                    let current_bidding_amount = details.current_bidding_amount.as_u128() as f64
                        / TEN.pow(details.decimals_bidding_token).as_u128() as f64;
                    let auctioning_token = details.address_auctioning_token;
                    let bidding_token_address = details.address_bidding_token;
                    let empty_stable_coin_list: Vec<Address> = Vec::new();
                    let legit_stable_coins = LEGIT_STABLE_COINS
                        .get(&chain_id)
                        .unwrap_or(&empty_stable_coin_list);
                    let weth = PRICE_FEED_SUPPORTED_TOKENS
                        .get(&chain_id)
                        .unwrap_or(&empty_stable_coin_list);
                    if legit_stable_coins.contains(&bidding_token_address) {
                        current_bidding_amount
                    } else if legit_stable_coins.contains(&auctioning_token) {
                        (current_bidding_amount) / (details.current_clearing_price as f64)
                    } else if weth.contains(&bidding_token_address) {
                        let eth_price = the_graph_reader
                            .get_eth_usd_price(details.end_time_timestamp)
                            .await?;
                        current_bidding_amount * eth_price
                    } else {
                        0f64
                    }
                }
                None => 0f64,
            };
        }
        {
            let mut auction_details_hashmap = self.auction_details.write().await;
            match auction_details_hashmap.entry(auction_id) {
                Entry::Occupied(mut details) => {
                    details.get_mut().usd_amount_traded = usd_amount;
                }
                Entry::Vacant(_) => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use primitive_types::U256;

    #[tokio::test(flavor = "current_thread")]
    async fn adds_order_to_orderbook() {
        let user_id = 10_u64;
        let order = Order {
            sell_amount: U256::from_dec_str("1230").unwrap(),
            buy_amount: U256::from_dec_str("123").unwrap(),
            user_id,
        };
        let auction_id = 1;
        let orderbook = Orderbook::new();
        orderbook
            .set_auction_details(auction_id, AuctionDetails::default())
            .await
            .unwrap();
        orderbook.insert_orders(auction_id, vec![order]).await;
        assert_eq!(orderbook.get_orders(auction_id).await, vec![order]);
        let mut expected_hash_set = HashSet::new();
        expected_hash_set.insert(auction_id);
        assert_eq!(
            orderbook.get_used_auctions(user_id).await,
            expected_hash_set
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn sorts_orders_from_orderbook() {
        let order_1 = Order {
            sell_amount: U256::from_dec_str("1230").unwrap(),
            buy_amount: U256::from_dec_str("123").unwrap(),
            user_id: 10_u64,
        };
        let auction_id = 1;
        let orderbook = Orderbook::new();
        orderbook
            .set_auction_details(auction_id, AuctionDetails::default())
            .await
            .unwrap();
        orderbook.insert_orders(auction_id, vec![order_1]).await;
        let order_2 = Order {
            sell_amount: U256::from_dec_str("1230").unwrap(),
            buy_amount: U256::from_dec_str("12").unwrap(),
            user_id: 10_u64,
        };
        orderbook.insert_orders(auction_id, vec![order_2]).await;
        orderbook.sort_orders(auction_id).await;
        assert_eq!(
            orderbook.get_orders(auction_id).await,
            vec![order_2, order_1]
        );
    }
    #[tokio::test(flavor = "current_thread")]
    async fn get_clearing_order_and_price_() {
        let order_1 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 1_u64,
        };
        let order_2 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("1").unwrap(),
            user_id: 2_u64,
        };
        let order_3 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("3").unwrap(),
            user_id: 3_u64,
        };
        let initial_order = Order {
            sell_amount: U256::from_dec_str("4").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 10_u64,
        };
        let auction_id = 1;
        let mut orderbook = Orderbook::new();
        orderbook
            .set_auction_details(auction_id, AuctionDetails::default())
            .await
            .unwrap();
        orderbook
            .insert_orders(auction_id, vec![order_1, order_2, order_3])
            .await;
        orderbook
            .update_initial_order(auction_id, initial_order)
            .await;
        orderbook.sort_orders(auction_id).await;
        let result = orderbook
            .get_clearing_order_and_volume(auction_id)
            .await
            .unwrap();

        assert_eq!(result.0, order_1);
        assert_eq!(result.1, order_1.sell_amount);
    }
    #[tokio::test(flavor = "current_thread")]
    async fn get_clearing_order_and_price_2() {
        let order_1 = Order {
            sell_amount: U256::from_dec_str("500000000000000000000").unwrap(),
            buy_amount: U256::from_dec_str("364697301239970824").unwrap(),
            user_id: 1_u64,
        };
        let order_2 = Order {
            sell_amount: U256::from_dec_str("500000000000000000000").unwrap(),
            buy_amount: U256::from_dec_str("334697301239970824").unwrap(),
            user_id: 2_u64,
        };
        let order_3 = Order {
            sell_amount: U256::from_dec_str("10000000000000000000").unwrap(),
            buy_amount: U256::from_dec_str("30697301239970824").unwrap(),
            user_id: 3_u64,
        };
        let order_4 = Order {
            sell_amount: U256::from_dec_str("500000000000000000000").unwrap(),
            buy_amount: U256::from_dec_str("374697301239970824").unwrap(),
            user_id: 3_u64,
        };
        let initial_order = Order {
            sell_amount: U256::from_dec_str("1000000000000000000").unwrap(),
            buy_amount: U256::from_dec_str("1300000000000000000000").unwrap(),
            user_id: 10_u64,
        };
        let auction_id = 1;
        let mut orderbook = Orderbook::new();
        orderbook
            .set_auction_details(auction_id, AuctionDetails::default())
            .await
            .unwrap();
        orderbook
            .insert_orders(auction_id, vec![order_1, order_2, order_3, order_4])
            .await;
        orderbook
            .update_initial_order(auction_id, initial_order)
            .await;
        orderbook.sort_orders(auction_id).await;
        let result = orderbook
            .get_clearing_order_and_volume(auction_id)
            .await
            .unwrap();

        assert_eq!(result.0, order_4);
    }
    #[tokio::test(flavor = "current_thread")]
    async fn get_previous_order() {
        let order_1 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 10_u64,
        };
        let auction_id = 1;
        let orderbook = Orderbook::new();
        assert_eq!(
            orderbook.get_previous_order(auction_id, order_1).await,
            *QUEUE_START
        );
        let order_2 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("3").unwrap(),
            user_id: 10_u64,
        };
        let order_3 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("3").unwrap(),
            user_id: 9_u64,
        };

        orderbook
            .set_auction_details(auction_id, AuctionDetails::default())
            .await
            .unwrap();
        orderbook
            .insert_orders(auction_id, vec![order_1, order_3])
            .await;
        orderbook
            .remove_claimed_orders(auction_id, vec![order_3])
            .await;
        assert_eq!(
            orderbook.get_previous_order(auction_id, order_2).await,
            order_1
        );
        orderbook
            .remove_claimed_orders(auction_id, vec![order_1])
            .await;
        assert_eq!(
            orderbook.get_previous_order(auction_id, order_2).await,
            *QUEUE_START
        );
    }
}
