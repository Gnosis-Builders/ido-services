use contracts::EasyAuction;
use ethcontract::{Address, H160};
use lazy_static::lazy_static;
use maplit::hashmap;
use orderbook::database::Database;
use orderbook::event_reader::EventReader;
use orderbook::health::HealthReporting;
use orderbook::health::HttpHealthEndpoint;
use orderbook::orderbook::Orderbook;
use orderbook::serve_task;
use orderbook::subgraph::uniswap_graph_api::UniswapSubgraphClient;
use primitive_types::H256;
use std::num::ParseFloatError;
use std::sync::Arc;
use std::{collections::HashMap, str::FromStr};
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

    /// Url of the Postgres database. By default connects to locally running postgres.
    #[structopt(long, env = "DB_URL", default_value = "postgresql://")]
    db_url: Url,

    /// The Ethereum node URL to connect to.
    #[structopt(
        long,
        env = "NODE_URL",
        default_value = "https://dev-openethereum.rinkeby.gnosisdev.com"
    )]
    pub node_url: Url,

    /// Number of blocks to sync in bulk.
    #[structopt(
        long,
        env = "NUMBER_OF_BLOCKS_TO_SYNC_PER_REQUEST",
        default_value = "10000"
    )]
    pub number_of_blocks_to_sync_per_request: u64,

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
// Todo: duplication from build file.
lazy_static! {
    pub static ref EASY_AUCTION_DEPLOYMENT_INFO: HashMap::<u32, (Address, Option<H256>)> = hashmap! {
        1 => (Address::from_str("0b7fFc1f4AD541A4Ed16b40D8c37f0929158D101").unwrap(), Some("0xa7ad659a9762720bd86a30b49a3e139928cc2a27d0863ab78110e19d2bef8a51".parse().unwrap())),
        4 => (Address::from_str("C5992c0e0A3267C7F75493D0F717201E26BE35f7").unwrap(), Some("0xbdd1dde815a908d407ec89fa9bc317d9e33621ccc6452ac0eb00fe2ed0d81ff4".parse().unwrap())),
        100 => (Address::from_str("0b7fFc1f4AD541A4Ed16b40D8c37f0929158D101").unwrap(), Some("0x5af5443ba9add113a42b0219ac8f398c383dc5a3684a221fd24c5655b8316931".parse().unwrap())),
        137 => (Address::from_str("0b7fFc1f4AD541A4Ed16b40D8c37f0929158D101").unwrap(), Some("0x6093f70c46350202181e9b0edfcf8f0e966ddddeb8b24e8b73dd2ab636c1ce87".parse().unwrap())),
    };
}

pub async fn orderbook_maintenance(
    orderbook_latest: Arc<Orderbook>,
    orderbook_reorg_protected: Arc<Orderbook>,
    event_reader: EventReader,
    mut the_graph_reader: UniswapSubgraphClient,
    health: Arc<HttpHealthEndpoint>,
) -> ! {
    // First block considered for synchronization should be the one, in which the deployment
    // of Gnosis Auction contract happens
    let chain_id = event_reader.web3.eth().chain_id().await.unwrap();
    let tx_info = event_reader
        .web3
        .eth()
        .transaction(
            EASY_AUCTION_DEPLOYMENT_INFO
                .clone()
                .get(&chain_id.as_u32())
                .unwrap_or(&(Address::zero(), None))
                .1
                .unwrap()
                .into(),
        )
        .await
        .unwrap();
    let mut last_block_considered_for_reorg_protected_orderbook = 0u64;
    match tx_info {
        Some(tx) => {
            last_block_considered_for_reorg_protected_orderbook = tx.block_number.unwrap().as_u64()
        }
        None => tracing::error!("Deployment block was not found"),
    }

    let mut fully_indexed_events = false;
    loop {
        tracing::debug!("running order book maintenance with reorg protection");
        orderbook_reorg_protected
            .run_maintenance(
                &event_reader,
                &mut the_graph_reader,
                &mut last_block_considered_for_reorg_protected_orderbook,
                true,
                chain_id.as_u32(),
            )
            .await
            .expect("maintenance function not successful");
        // most ridiculous swap: Resetting the orderbook_latest to orderbook_protected
        {
            {
                let mut orderbook = orderbook_latest.orders.write().await;
                let orderbook_reorg_save = orderbook_reorg_protected.orders.read().await;
                orderbook.retain(|&k, _| k == 0);
                for auction_id in orderbook_reorg_save.keys() {
                    orderbook.insert(
                        *auction_id,
                        orderbook_reorg_save.get(auction_id).unwrap().clone(),
                    );
                }
            }
            {
                let mut orderbook = orderbook_latest.orders_display.write().await;
                let orderbook_reorg_save = orderbook_reorg_protected.orders_display.read().await;
                orderbook.retain(|&k, _| k == 0);
                for auction_id in orderbook_reorg_save.keys() {
                    orderbook.insert(
                        *auction_id,
                        orderbook_reorg_save.get(auction_id).unwrap().clone(),
                    );
                }
            }
            {
                let mut orderbook = orderbook_latest.orders_without_claimed.write().await;
                let orderbook_reorg_save = orderbook_reorg_protected
                    .orders_without_claimed
                    .read()
                    .await;
                orderbook.retain(|&k, _| k == 0);
                for auction_id in orderbook_reorg_save.keys() {
                    orderbook.insert(
                        *auction_id,
                        orderbook_reorg_save.get(auction_id).unwrap().clone(),
                    );
                }
            }
            {
                let mut orderbook = orderbook_latest.auction_details.write().await;
                let orderbook_reorg_save = orderbook_reorg_protected.auction_details.read().await;
                orderbook.retain(|&k, _| k == 0);
                for auction_id in orderbook_reorg_save.keys() {
                    orderbook.insert(
                        *auction_id,
                        orderbook_reorg_save.get(auction_id).unwrap().clone(),
                    );
                }
            }
            {
                let mut users = orderbook_latest.users.write().await;
                let users_reorg_save = orderbook_reorg_protected.users.read().await;
                users.retain(|&k, _| k == H160::zero());
                for address in users_reorg_save.keys() {
                    users.insert(*address, *users_reorg_save.get(address).unwrap());
                }
            }
            {
                let mut auction_participation =
                    orderbook_latest.auction_participation.write().await;
                let auction_participation_reorg_save =
                    orderbook_reorg_protected.auction_participation.read().await;
                auction_participation.retain(|&k, _| k == 0);
                for user_ids in auction_participation_reorg_save.keys() {
                    auction_participation.insert(
                        *user_ids,
                        auction_participation_reorg_save
                            .get(user_ids)
                            .unwrap()
                            .clone(),
                    );
                }
            }
        }
        let mut last_block_considered = last_block_considered_for_reorg_protected_orderbook; // Values are cloned, as we don't wanna store the values.
        orderbook_latest
            .run_maintenance(
                &event_reader,
                &mut the_graph_reader,
                &mut last_block_considered,
                false,
                chain_id.as_u32(),
            )
            .await
            .expect("maintenance function not successful");

        let current_block = event_reader
            .web3
            .eth()
            .block_number()
            .await
            .unwrap_or_else(|_| web3::types::U64::zero())
            .as_u64();

        if current_block == last_block_considered {
            health.notify_ready();
            fully_indexed_events = true;
            tracing::debug!("Orderbook fully synced");
        }
        if fully_indexed_events {
            tokio::time::delay_for(MAINTENANCE_INTERVAL).await;
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Arguments::from_args();
    tracing_setup::initialize(args.log_filter.as_str());
    tracing::debug!("running order book with {:#?}", args);
    let transport =
        web3::transports::Http::new(args.node_url.as_str()).expect("transport creation failed");
    let web3 = web3::Web3::new(transport);
    let easy_auction_contract = EasyAuction::deployed(&web3)
        .await
        .expect("Couldn't load deployed easyAuction");
    let event_reader = EventReader::new(
        easy_auction_contract,
        web3,
        args.number_of_blocks_to_sync_per_request,
    );
    let database = Database::new(args.db_url.as_str()).expect("failed to create database");
    let orderbook_latest = Arc::new(Orderbook::new());
    let orderbook_reorg_save = Arc::new(Orderbook::new());
    let the_graph_reader = UniswapSubgraphClient::for_chain(1).unwrap();
    let health = Arc::new(HttpHealthEndpoint::new());
    let serve_task = serve_task(
        orderbook_latest.clone(),
        database,
        health.clone(),
        args.bind_address,
    );
    let maintenance_task = task::spawn(orderbook_maintenance(
        orderbook_latest,
        orderbook_reorg_save,
        event_reader,
        the_graph_reader,
        health,
    ));
    tokio::select! {
        result = serve_task => tracing::error!(?result, "serve task exited"),
        result = maintenance_task => tracing::error!(?result, "maintenance task exited"),
    };
}

pub fn duration_from_seconds(s: &str) -> Result<Duration, ParseFloatError> {
    Ok(Duration::from_secs_f32(s.parse()?))
}
