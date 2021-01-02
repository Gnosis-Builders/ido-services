use contracts::EasyAuction;
use orderbook::event_reader::EventReader;
use orderbook::orderbook::Orderbook;
use orderbook::serve_task;
use std::collections::HashMap;
use std::num::ParseFloatError;
use std::sync::Arc;
use std::{net::SocketAddr, time::Duration};
use structopt::StructOpt;
use tokio::task;
use url::Url;

#[derive(Debug, StructOpt)]
struct Arguments {
    #[structopt(
        long,
        env = "LOG_FILTER",
        default_value = "warn,orderbook=debug,solver=debug"
    )]
    pub log_filter: String,

    #[structopt(long, env = "BIND_ADDRESS", default_value = "0.0.0.0:8080")]
    bind_address: SocketAddr,

    /// The Ethereum node URL to connect to.
    #[structopt(
        long,
        env = "NODE_URL",
        default_value = "https://dev-openethereum.rinkeby.gnosisdev.com"
    )]
    pub node_url: Url,

    /// Timeout for web3 operations on the node in seconds.
    #[structopt(
                long,
                env = "NODE_TIMEOUT",
                default_value = "5",
                parse(try_from_str = duration_from_seconds),
            )]
    pub node_timeout: Duration,
}

const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(3);

pub async fn orderbook_maintenance(
    orderbook_latest: Arc<Orderbook>,
    orderbook_reorg_protected: Arc<Orderbook>,
    event_reader: EventReader,
) -> ! {
    let mut last_block_considered_for_reorg_protected_orderbook = HashMap::new();
    loop {
        tracing::debug!("running order book maintenance with reorg protection");
        orderbook_reorg_protected
            .run_maintenance(
                &event_reader,
                &mut last_block_considered_for_reorg_protected_orderbook,
                true,
            )
            .await
            .expect("maintenance function not successful");
        // most ridiculous swap: Resetting the orderbook_latest to orderbook_protected
        {
            let mut orderbook = orderbook_latest.orders.write().await;
            let orderbook_reorg_save = orderbook_reorg_protected.orders.read().await;
            for auction_id in orderbook_reorg_save.keys() {
                orderbook.insert(
                    *auction_id,
                    orderbook_reorg_save.get(auction_id).unwrap().clone(),
                );
            }
            let mut users = orderbook_latest.users.write().await;
            let users_reorg_save = orderbook_reorg_protected.users.read().await;
            for address in users_reorg_save.keys() {
                users.insert(*address, users_reorg_save.get(address).unwrap().clone());
            }
        }
        orderbook_latest
            .run_maintenance(
                &event_reader,
                &mut last_block_considered_for_reorg_protected_orderbook.clone(), // Values are cloned, as we don't wanna store the values.
                false,
            )
            .await
            .expect("maintenance function not successful");

        tokio::time::delay_for(MAINTENANCE_INTERVAL).await;
    }
}

#[tokio::main]
async fn main() {
    let args = Arguments::from_args();
    tracing_setup::initialize(args.log_filter.as_str());
    tracing::info!("running order book with {:#?}", args);
    let transport =
        web3::transports::Http::new(args.node_url.as_str()).expect("transport creation failed");
    let web3 = web3::Web3::new(transport);
    let easy_auction_contract = EasyAuction::deployed(&web3)
        .await
        .expect("Couldn't load deployed easyAuction");
    let event_reader = EventReader::new(easy_auction_contract, web3);
    let orderbook_latest = Arc::new(Orderbook::new());
    let orderbook_reorg_save = Arc::new(Orderbook::new());
    let serve_task = serve_task(orderbook_latest.clone(), args.bind_address);
    let maintenance_task = task::spawn(orderbook_maintenance(
        orderbook_latest,
        orderbook_reorg_save,
        event_reader,
    ));
    tokio::select! {
        result = serve_task => tracing::error!(?result, "serve task exited"),
        result = maintenance_task => tracing::error!(?result, "maintenance task exited"),
    };
}

pub fn duration_from_seconds(s: &str) -> Result<Duration, ParseFloatError> {
    Ok(Duration::from_secs_f32(s.parse()?))
}
