mod filter;
mod handler;

use crate::orderbook::Orderbook;
use std::sync::Arc;
use warp::Filter;

pub fn handle_all_routes(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let get_previous_order = filter::get_previous_order(orderbook.clone());
    let get_order_book_display_data = filter::get_order_book_display_data(orderbook.clone());
    let get_user_orders = filter::get_user_orders(orderbook);
    warp::path!("api" / "v1" / ..).and(
        get_previous_order
            .or(get_order_book_display_data)
            .or(get_user_orders),
    )
}
