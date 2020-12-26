use contracts::EasyAuction;
use orderbook::orderbook::Orderbook;
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
        default_value = "https://dev-openethereum.rinkeby.gnosisdev.com:8545"
    )]
    pub node_url: Url,

    /// Timeout for web3 operations on the node in seconds.
    #[structopt(
                long,
                env = "NODE_TIMEOUT",
                default_value = "10",
                parse(try_from_str = duration_from_seconds),
            )]
    pub node_timeout: Duration,
}

const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(10);

pub async fn orderbook_maintenance(
    orderbook: Arc<Orderbook>,
    orderbook_reorg_protected: Arc<Orderbook>,
    contract: EasyAuction,
) -> ! {
    loop {
        tracing::debug!("running order book maintenance");
        orderbook_reorg_protected
            .run_maintenance_with_reorg_protection(&contract)
            .await;
        orderbook.run_maintenance(&contract).await;
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
    let orderbook_latest = Arc::new(Orderbook::new());
    let orderbook_reorg_save = Arc::new(Orderbook::new());
    let maintenance_task = task::spawn(orderbook_maintenance(
        orderbook_latest,
        orderbook_reorg_save,
        easy_auction_contract,
    ));
    tokio::select! {
        result = maintenance_task => tracing::error!(?result, "maintenance task exited"),
    };
}

pub fn duration_from_seconds(s: &str) -> Result<Duration, ParseFloatError> {
    Ok(Duration::from_secs_f32(s.parse()?))
}
