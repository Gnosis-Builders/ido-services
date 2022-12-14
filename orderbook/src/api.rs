mod filter;
mod handler;

use crate::database::Database;
use crate::health::HttpHealthEndpoint;
use crate::orderbook::Orderbook;
use std::sync::Arc;
use warp::Filter;

pub fn handle_all_routes(
    orderbook: Arc<Orderbook>,
    database: Database,
    health: Arc<HttpHealthEndpoint>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let get_signature = filter::get_signature(database.clone());
    let provide_signatures_object = filter::provide_signatures_object(orderbook.clone(), database);
    let get_previous_order = filter::get_previous_order(orderbook.clone());
    let get_order_book_display_data = filter::get_order_book_display_data(orderbook.clone());
    let get_user_orders = filter::get_user_orders(orderbook.clone());
    let health_filter = filter::health_filter_readiness(health);
    let get_user_orders_without_claimed =
        filter::get_user_orders_without_canceled_or_claimed(orderbook.clone());
    let get_clearing_order_and_volume = filter::get_clearing_order_and_volume(orderbook.clone());
    let get_details_of_most_interesting_auctions =
        filter::get_details_of_most_interesting_auctions(orderbook.clone());
    let get_details_of_most_interesting_closed_auctions =
        filter::get_details_of_most_interesting_closed_auctions(orderbook.clone());
    let get_all_auction_with_details = filter::get_all_auction_with_details(orderbook.clone());
    let get_auction_with_details = filter::get_auction_with_details(orderbook.clone());
    let get_all_auction_with_details_with_user_participation =
        filter::get_all_auction_with_details_with_user_participation(orderbook);
    let api_routes = warp::path!("api" / "v1" / ..).and(
        get_previous_order
            .or(get_order_book_display_data)
            .or(get_user_orders)
            .or(get_user_orders_without_claimed)
            .or(get_clearing_order_and_volume)
            .or(get_details_of_most_interesting_auctions)
            .or(get_details_of_most_interesting_closed_auctions)
            .or(get_all_auction_with_details)
            .or(get_auction_with_details)
            .or(get_all_auction_with_details_with_user_participation)
            .or(get_signature)
            .or(provide_signatures_object),
    );
    let health_routes = warp::path!("health" / ..).and(health_filter);
    api_routes.or(health_routes)
}
