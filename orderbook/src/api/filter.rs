use super::handler;
use crate::orderbook::Orderbook;
use hex::{FromHex, FromHexError};
use model::order::Order;
use primitive_types::H160;
use std::{str::FromStr, sync::Arc};
use warp::Filter;

fn with_orderbook(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (Arc<Orderbook>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || orderbook.clone())
}
/// Wraps H160 with FromStr that can handle a `0x` prefix.
/// Unfortunately, it is public, since I was unable to map in filter get_user_orders
/// three arguments to three arguments .map(|auction_id, hash, orderbook| auction_id, hash.0, orderbook)
pub struct H160Wrapper(pub H160);
impl FromStr for H160Wrapper {
    type Err = FromHexError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        Ok(H160Wrapper(H160(FromHex::from_hex(s)?)))
    }
}
pub fn get_previous_order(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_previous_order" / u64 / Order)
        .and(warp::get())
        .and(with_orderbook(orderbook))
        .and_then(handler::get_previous_order)
}

pub fn get_user_orders(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_user_orders" / u64 / H160Wrapper)
        .and(warp::get())
        .and(with_orderbook(orderbook))
        .and_then(handler::get_user_orders)
}
pub fn get_clearing_order_and_volume(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_clearing_order_and_volume" / u64)
        .and(warp::get())
        .and(with_orderbook(orderbook))
        .and_then(handler::get_clearing_order_and_volume)
}

pub fn get_user_orders_without_canceled_or_claimed(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_user_orders_without_canceled_or_claimed" / u64 / H160Wrapper)
        .and(warp::get())
        .and(with_orderbook(orderbook))
        .and_then(handler::get_user_orders_without_canceled_or_claimed)
}

pub fn get_order_book_display_data(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_order_book_display_data" / u64)
        .and(warp::get())
        .and(with_orderbook(orderbook))
        .and_then(handler::get_order_book_display_data)
}

#[cfg(test)]
pub mod test_util {
    use super::*;
    use model::user::User;
    use primitive_types::U256;
    use warp::{http::StatusCode, test::request};

    #[tokio::test]
    async fn get_previous_order_() {
        let orderbook = Orderbook::default();
        let auction_id: u64 = 1;
        let order_1 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 10_u64,
        };
        let order_2 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("3").unwrap(),
            user_id: 10_u64,
        };
        orderbook.insert_orders(auction_id, vec![order_1]).await;
        let filter = get_previous_order(Arc::new(orderbook));
        let response = request()
            .path(&format!("/get_previous_order/{:}/{:}", auction_id, order_2))
            .method("GET")
            .reply(&filter)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response_order: Order = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(response_order, order_1);
    }

    #[tokio::test]
    async fn get_user_orders_() {
        let orderbook = Orderbook::default();
        let auction_id: u64 = 1;
        let order_1 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 10_u64,
        };
        let order_2 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 9_u64,
        };
        let user = User {
            address: "740a98F8f4fAe0986FB3264Fe4aaCf94ac1EE96f".parse().unwrap(),
            user_id: 10_u64,
        };
        orderbook
            .insert_orders(auction_id, vec![order_1, order_2])
            .await;
        orderbook.insert_users(vec![user]).await;
        let filter = get_user_orders(Arc::new(orderbook));
        println!(
            "{}",
            format!(
                "/get_user_orders/{:}/{:}",
                auction_id,
                user.show_full_address()
            )
        );
        let response = request()
            .path(&format!(
                "/get_user_orders/{:}/{:}",
                auction_id,
                user.show_full_address()
            ))
            .method("GET")
            .reply(&filter)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response_order: Vec<Order> = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(response_order, vec![order_1]);
    }

    #[tokio::test]
    async fn get_user_orders_without_canceled_or_claimed_() {
        let orderbook = Orderbook::default();
        let auction_id: u64 = 1;
        let order_1 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 10_u64,
        };
        let order_2 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 9_u64,
        };
        let user = User {
            address: "740a98F8f4fAe0986FB3264Fe4aaCf94ac1EE96f".parse().unwrap(),
            user_id: 10_u64,
        };
        orderbook
            .insert_orders(auction_id, vec![order_1, order_2])
            .await;
        orderbook.insert_users(vec![user]).await;

        let filter = get_user_orders_without_canceled_or_claimed(Arc::new(orderbook));
        println!(
            "{}",
            format!(
                "/get_user_orders_without_canceled_or_claimed/{:}/{:}",
                auction_id,
                user.show_full_address()
            )
        );
        let response = request()
            .path(&format!(
                "/get_user_orders_without_canceled_or_claimed/{:}/{:}",
                auction_id,
                user.show_full_address()
            ))
            .method("GET")
            .reply(&filter)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response_order: Vec<Order> = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(response_order, vec![order_1]);
    }
}
