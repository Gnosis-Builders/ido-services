use crate::api::filter::H160Wrapper;
use crate::orderbook::Orderbook;
use model::order::Order;
use std::{convert::Infallible, sync::Arc};
use warp::{
    http::StatusCode,
    reply::{json, with_status},
};

pub async fn get_previous_order(
    auction_id: u64,
    order: Order,
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let order = orderbook.get_previous_order(auction_id, order).await;
    Ok(with_status(json(&order), StatusCode::OK))
}

pub async fn get_user_orders(
    auction_id: u64,
    user: H160Wrapper,
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let order = orderbook.get_user_orders(auction_id, user.0).await;
    Ok(with_status(json(&order), StatusCode::OK))
}

pub async fn get_user_orders_without_canceled_or_claimed(
    auction_id: u64,
    user: H160Wrapper,
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let order = orderbook
        .get_user_orders_without_canceled_claimed(auction_id, user.0)
        .await;
    Ok(with_status(json(&order), StatusCode::OK))
}

pub async fn get_clearing_order_and_volume(
    auction_id: u64,
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let order = orderbook.get_clearing_order_and_volume(auction_id).await;
    Ok(with_status(json(&order), StatusCode::OK))
}

pub async fn get_order_book_display_data(
    auction_id: u64,
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let orderbook_data = orderbook.get_order_book_display(auction_id).await;
    match orderbook_data {
        Err(err) => Ok(with_status(
            json(&format!("{:}", err)),
            StatusCode::BAD_REQUEST,
        )),
        Ok(orderbook_data) => Ok(with_status(json(&orderbook_data), StatusCode::OK)),
    }
}

pub async fn get_details_of_most_interesting_auctions(
    number_of_auctions: u64,
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let auction_detail_data = orderbook
        .get_most_interesting_auctions(number_of_auctions)
        .await;
    match auction_detail_data {
        Err(err) => Ok(with_status(
            json(&format!("{:}", err)),
            StatusCode::BAD_REQUEST,
        )),
        Ok(auction_detail_data) => Ok(with_status(json(&auction_detail_data), StatusCode::OK)),
    }
}

pub async fn get_all_auction_with_details(
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let auction_detail_data = orderbook.get_all_auction_with_details().await;
    match auction_detail_data {
        Err(err) => Ok(with_status(
            json(&format!("{:}", err)),
            StatusCode::BAD_REQUEST,
        )),
        Ok(auction_detail_data) => Ok(with_status(json(&auction_detail_data), StatusCode::OK)),
    }
}
