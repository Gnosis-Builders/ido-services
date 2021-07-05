use contracts::{ERC20Mintable, EasyAuction};
use ethcontract::prelude::{Account, Address, BlockNumber, U256};
use model::order::PricePoint;
use orderbook::database::Database;
use orderbook::event_reader::EventReader;
use orderbook::health::HttpHealthEndpoint;
use orderbook::orderbook::{Orderbook, QUEUE_START};
use orderbook::subgraph::uniswap_graph_api::UniswapSubgraphClient;
use serde_json::Value;
use std::{str::FromStr, sync::Arc};

mod ganache;

const API_HOST: &str = "http://127.0.0.1:8080";
const ORDERBOOK_DISPLAY_ENDPOINT: &str = "/api/v1/get_order_book_display_data/";

#[tokio::test(flavor = "current_thread")]
async fn event_parsing() {
    ganache::test(|web3| async {
        tracing_setup::initialize("debug");

        let accounts: Vec<Address> = web3.eth().accounts().await.expect("get accounts failed");
        let chain_id = web3.eth().chain_id().await.unwrap();

        let auctioneer = Account::Local(accounts[0], None);
        let trader_a = Account::Local(accounts[1], None);

        let deploy_mintable_token = || async {
            ERC20Mintable::builder(&web3, String::from("TEST"), String::from("18"))
                .gas(8_000_000u32.into())
                .deploy()
                .await
                .expect("MintableERC20 deployment failed")
        };

        macro_rules! tx {
            ($acc:ident, $call:expr) => {{
                const NAME: &str = stringify!($call);
                $call
                    .from($acc.clone())
                    .gas(8_000_000u32.into())
                    .send()
                    .await
                    .expect(&format!("{} failed", NAME))
            }};
        }

        // Fetch deployed instances
        let easy_auction = EasyAuction::at(
            &web3,
            "5b1869d9a4c187f2eaa108f3062412ecf0526b24".parse().unwrap(),
        );
        let auction_id = U256::from_dec_str("1").unwrap();
        // Create & Mint tokens to trade
        let token_a = deploy_mintable_token().await;
        tx!(auctioneer, token_a.mint(auctioneer.address(), to_wei(100)));

        let token_b = deploy_mintable_token().await;
        tx!(auctioneer, token_b.mint(trader_a.address(), to_wei(100)));

        // Initiate auction
        tx!(
            auctioneer,
            token_a.approve(easy_auction.address(), to_wei(100))
        );
        let mut current_time_stamp = Some(0u64);
        let block_info = web3
            .eth()
            .block(ethcontract::BlockId::Number(BlockNumber::Latest))
            .await
            .unwrap();
        if let Some(block_data) = block_info {
            current_time_stamp = Some(block_data.timestamp.as_u64());
        }
        tx!(
            auctioneer,
            easy_auction.initiate_auction(
                token_a.address(),
                token_b.address(),
                U256::from(current_time_stamp.unwrap())
                    .checked_add(U256::from_str("3600").unwrap())
                    .unwrap(),
                U256::from(current_time_stamp.unwrap())
                    .checked_add(U256::from_str("3600").unwrap())
                    .unwrap(),
                (10_u128).checked_pow(18).unwrap(),
                (10_u128).checked_pow(18).unwrap(),
                U256::from_str("1").unwrap(),
                U256::from_str("1").unwrap(),
                false,
                Address::zero(),
                ethcontract::Bytes(Vec::new()),
            )
        );
        // Place Order
        tx!(
            trader_a,
            token_b.approve(easy_auction.address(), to_wei(100))
        );
        let mut queue_start_as_hex = [0u8; 32];
        hex::decode_to_slice(
            QUEUE_START.to_string().strip_prefix("0x").unwrap(),
            &mut queue_start_as_hex,
        )
        .unwrap();
        tx!(
            trader_a,
            easy_auction.place_sell_orders(
                auction_id,
                vec![(10_u128).checked_pow(18).unwrap()],
                vec![(10_u128).checked_pow(18).unwrap().checked_mul(2).unwrap()],
                vec![ethcontract::Bytes(queue_start_as_hex)],
                ethcontract::Bytes(vec![0u8]),
            )
        );

        // serve task
        let orderbook = Arc::new(Orderbook::new());
        let database = Database::new(&"postgresql://").expect("failed to create database");
        database.clear().await.unwrap();
        let health = Arc::new(HttpHealthEndpoint::new());
        orderbook::serve_task(
            orderbook.clone(),
            database,
            health,
            API_HOST[7..].parse().expect("Couldn't parse API address"),
        );
        let event_reader = EventReader::new(easy_auction, web3, 100u64);
        let mut last_block_considered = 1u64;
        let mut the_graph_reader = UniswapSubgraphClient::for_chain(1).unwrap();

        orderbook::orderbook::Orderbook::run_maintenance(
            &orderbook,
            &event_reader,
            &mut the_graph_reader,
            &mut last_block_considered,
            false,
            chain_id.as_u32(),
        )
        .await
        .unwrap();
        let client = reqwest::Client::new();

        let orderbook_display = client
            .get(&format!(
                "{}{}{}",
                API_HOST, ORDERBOOK_DISPLAY_ENDPOINT, auction_id
            ))
            .send()
            .await
            .unwrap();
        let orderbook_value: Value =
            serde_json::from_str(&orderbook_display.text().await.unwrap()).unwrap();
        let expected_price_point = PricePoint {
            price: 2.0_f64,
            volume: 2.0_f64,
        };
        let bids: Vec<PricePoint> =
            serde_json::from_value(orderbook_value["bids"].clone()).unwrap();
        assert_eq!(bids, vec![expected_price_point]);
        let mut the_graph_reader = UniswapSubgraphClient::for_chain(1).unwrap();

        //rerunning the maintenance function should not change the result
        orderbook::orderbook::Orderbook::run_maintenance(
            &orderbook,
            &event_reader,
            &mut the_graph_reader,
            &mut last_block_considered,
            false,
            chain_id.as_u32(),
        )
        .await
        .unwrap();
        let orderbook_display = client
            .get(&format!(
                "{}{}{}",
                API_HOST, ORDERBOOK_DISPLAY_ENDPOINT, auction_id
            ))
            .send()
            .await
            .unwrap();
        let orderbook: Value =
            serde_json::from_str(&orderbook_display.text().await.unwrap()).unwrap();
        let bids: Vec<PricePoint> = serde_json::from_value(orderbook["bids"].clone()).unwrap();
        assert_eq!(bids, vec![expected_price_point]);
    })
    .await;
}

fn to_wei(base: u32) -> U256 {
    U256::from(base) * U256::from(10).pow(18.into())
}
