use super::handler;
use crate::orderbook::Orderbook;
use model::order::Order;
use std::sync::Arc;
use warp::Filter;

fn with_orderbook(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (Arc<Orderbook>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || orderbook.clone())
}

pub fn get_previous_order(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_previous_order" / u64 / Order)
        .and(warp::get())
        .and(with_orderbook(orderbook))
        .and_then(handler::get_previous_order)
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
    use primitive_types::U256;
    use warp::{http::StatusCode, test::request};

    #[tokio::test]
    async fn get_previous_order_() {
        let mut orderbook = Orderbook::default();
        let auction_id: u64 = 1;
        let order_1 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 10 as u64,
        };
        let order_2 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("3").unwrap(),
            user_id: 10 as u64,
        };
        orderbook.insert_order(auction_id, order_1).await;
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
}
