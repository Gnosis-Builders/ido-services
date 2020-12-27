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
