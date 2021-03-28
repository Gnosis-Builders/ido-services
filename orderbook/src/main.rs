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
    static ref EASY_AUCTION_DEPLOYMENT_INFO: HashMap::<u32, (Address, Option<H256>)> = hashmap! {
    4 => (Address::from_str("307C1384EFeF241d6CBBFb1F85a04C54307Ac9F6").unwrap(), Some("0xecf8358d08dfdbd9549c0affa2226b062fe78867f156a258bd9da1e05ad842aa".parse().unwrap())),
    100 => (Address::from_str("9BacE46438b3f3e0c06d67f5C1743826EE8e87DA").unwrap(), Some("0x7304d6dfe40a8b5a97c6579743733139dd50c3b4a7d39181fd7c24ac28c3986f".parse().unwrap())),
    };
}

pub async fn orderbook_maintenance(
    orderbook_latest: Arc<Orderbook>,
    orderbook_reorg_protected: Arc<Orderbook>,
    event_reader: EventReader,
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
            orderbook.retain(|&k, _| k == 0);
            for auction_id in orderbook_reorg_save.keys() {
                orderbook.insert(
                    *auction_id,
                    orderbook_reorg_save.get(auction_id).unwrap().clone(),
                );
            }
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
            let mut orderbook = orderbook_latest.auction_details.write().await;
            let orderbook_reorg_save = orderbook_reorg_protected.auction_details.read().await;
            orderbook.retain(|&k, _| k == 0);
            for auction_id in orderbook_reorg_save.keys() {
                orderbook.insert(
                    *auction_id,
                    orderbook_reorg_save.get(auction_id).unwrap().clone(),
                );
            }
            let mut users = orderbook_latest.users.write().await;
            let users_reorg_save = orderbook_reorg_protected.users.read().await;
            users.retain(|&k, _| k == H160::zero());
            for address in users_reorg_save.keys() {
                users.insert(*address, *users_reorg_save.get(address).unwrap());
            }
            let mut auction_participation = orderbook_latest.auction_participation.write().await;
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
        let mut last_block_considered = last_block_considered_for_reorg_protected_orderbook; // Values are cloned, as we don't wanna store the values.
        orderbook_latest
            .run_maintenance(&event_reader, &mut last_block_considered, false)
            .await
            .expect("maintenance function not successful");

        let current_block = event_reader
            .web3
            .eth()
            .block_number()
            .await
            .unwrap_or_else(|_| web3::types::U64::zero())
            .as_u64();
        tokio::time::delay_for(MAINTENANCE_INTERVAL).await;
        if current_block == last_block_considered {
            health.notify_ready();
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
    let event_reader = EventReader::new(easy_auction_contract, web3);
    let database = Database::new(args.db_url.as_str()).expect("failed to create database");
    let orderbook_latest = Arc::new(Orderbook::new());
    let orderbook_reorg_save = Arc::new(Orderbook::new());
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
