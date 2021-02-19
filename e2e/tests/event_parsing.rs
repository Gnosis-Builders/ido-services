use contracts::{ERC20Mintable, EasyAuction};
use ethcontract::prelude::{Account, Address, Http, Web3, U256};
use model::order::PricePoint;
use orderbook::event_reader::EventReader;
use orderbook::orderbook::{Orderbook, QUEUE_START};
use serde_json::Value;
use std::{str::FromStr, sync::Arc};

const NODE_HOST: &str = "http://127.0.0.1:8545";
const API_HOST: &str = "http://127.0.0.1:8080";
const ORDERBOOK_DISPLAY_ENDPOINT: &str = "/api/v1/get_order_book_display_data/";

#[tokio::test]
async fn test_with_ganache() {
    tracing_setup::initialize("debug");
    let http = Http::new(NODE_HOST).expect("transport failure");
    let web3 = Web3::new(http);

    let accounts: Vec<Address> = web3.eth().accounts().await.expect("get accounts failed");
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
        "e78a0f7e598cc8b0bb87894b0f60dd2a88d6a8ab".parse().unwrap(),
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
    tx!(
        auctioneer,
        easy_auction.initiate_auction(
            token_a.address(),
            token_b.address(),
            U256::from_str("3600").unwrap(),
            U256::from_str("3600").unwrap(),
            (10_u128).checked_pow(18).unwrap(),
            (10_u128).checked_pow(18).unwrap(),
            U256::from_str("1").unwrap(),
            U256::from_str("1").unwrap(),
            false,
            Address::zero()
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
            vec![queue_start_as_hex],
            vec![0u8],
        )
    );

    // serve task
    let orderbook = Arc::new(Orderbook::new());
    orderbook::serve_task(
        orderbook.clone(),
        API_HOST[7..].parse().expect("Couldn't parse API address"),
    );
    let event_reader = EventReader::new(easy_auction, web3);
    let mut last_block_considered_hashmap = std::collections::HashMap::new();
    orderbook::orderbook::Orderbook::run_maintenance(
        &orderbook,
        &event_reader,
        &mut last_block_considered_hashmap,
        false,
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
    let bids: Vec<PricePoint> = serde_json::from_value(orderbook_value["bids"].clone()).unwrap();
    assert_eq!(bids, vec![expected_price_point]);

    //rerunning the maintenance function should not change the result

    orderbook::orderbook::Orderbook::run_maintenance(
        &orderbook,
        &event_reader,
        &mut last_block_considered_hashmap,
        false,
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
    let orderbook: Value = serde_json::from_str(&orderbook_display.text().await.unwrap()).unwrap();
    let bids: Vec<PricePoint> = serde_json::from_value(orderbook["bids"].clone()).unwrap();
    assert_eq!(bids, vec![expected_price_point]);
}

fn to_wei(base: u32) -> U256 {
    U256::from(base) * U256::from(10).pow(18.into())
}
