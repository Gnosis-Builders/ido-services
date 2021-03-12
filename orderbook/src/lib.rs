pub mod api;
pub mod event_reader;
pub mod orderbook;
pub mod signatures;

use crate::orderbook::Orderbook;
use crate::signatures::SignatureStore;
use std::{net::SocketAddr, sync::Arc};
use tokio::{task, task::JoinHandle};
use warp::Filter;

pub fn serve_task(
    orderbook: Arc<Orderbook>,
    signatures: Arc<SignatureStore>,
    address: SocketAddr,
) -> JoinHandle<()> {
    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST", "DELETE", "OPTIONS", "PUT", "PATCH"])
        .allow_headers(vec!["Origin", "Content-Type", "X-Auth-Token", "X-AppId"]);
    let filter = api::handle_all_routes(orderbook, signatures).with(cors);
    tracing::info!(%address, "serving order book");
    task::spawn(warp::serve(filter).bind(address))
}
