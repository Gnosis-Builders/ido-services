use super::handler;
use crate::api::handler::extract_signatures_object_from_json;
use crate::database::Database;
use crate::health::HttpHealthEndpoint;
use crate::orderbook::Orderbook;
use hex::{FromHex, FromHexError};
use model::order::Order;
use primitive_types::H160;
use std::{str::FromStr, sync::Arc};
use warp::Filter;

fn with_orderbook(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (Arc<Orderbook>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || orderbook.clone())
}

fn with_health(
    health: Arc<HttpHealthEndpoint>,
) -> impl Filter<Extract = (Arc<HttpHealthEndpoint>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || health.clone())
}

fn with_signatures(
    db: Database,
) -> impl Filter<Extract = (Database,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || db.clone())
}
/// Wraps H160 with FromStr that can handle a `0x` prefix.
/// Unfortunately, it is public, since I was unable to map in filter get_user_orders
/// three arguments to three arguments .map(|auction_id, hash, orderbook| auction_id, hash.0, orderbook)
pub struct H160Wrapper(pub H160);
impl FromStr for H160Wrapper {
    type Err = FromHexError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        Ok(H160Wrapper(H160(FromHex::from_hex(s)?)))
    }
}
pub fn get_previous_order(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_previous_order" / u64 / Order)
        .and(warp::get())
        .and(with_orderbook(orderbook))
        .and_then(handler::get_previous_order)
}

pub fn health_filter_readiness(
    health: Arc<HttpHealthEndpoint>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("readiness")
        .and(warp::get())
        .and(with_health(health))
        .and_then(handler::readiness)
}

pub fn get_user_orders(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_user_orders" / u64 / H160Wrapper)
        .and(warp::get())
        .and(with_orderbook(orderbook))
        .and_then(handler::get_user_orders)
}
pub fn get_clearing_order_and_volume(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_clearing_order_and_volume" / u64)
        .and(warp::get())
        .and(with_orderbook(orderbook))
        .and_then(handler::get_clearing_order_and_volume)
}

pub fn get_user_orders_without_canceled_or_claimed(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_user_orders_without_canceled_or_claimed" / u64 / H160Wrapper)
        .and(warp::get())
        .and(with_orderbook(orderbook))
        .and_then(handler::get_user_orders_without_canceled_or_claimed)
}

pub fn get_order_book_display_data(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_order_book_display_data" / u64)
        .and(warp::get())
        .and(with_orderbook(orderbook))
        .and_then(handler::get_order_book_display_data)
}

pub fn get_details_of_most_interesting_auctions(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_details_of_most_interesting_auctions" / u64)
        .and(warp::get())
        .and(with_orderbook(orderbook))
        .and_then(handler::get_details_of_most_interesting_auctions)
}

pub fn get_details_of_most_interesting_closed_auctions(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_details_of_most_interesting_closed_auctions" / u64)
        .and(warp::get())
        .and(with_orderbook(orderbook))
        .and_then(handler::get_details_of_most_interesting_closed_auctions)
}

pub fn get_auction_with_details(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_auction_with_details" / u64)
        .and(with_orderbook(orderbook))
        .and_then(handler::get_auction_with_details)
}

pub fn get_all_auction_with_details(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_all_auction_with_details")
        .and(with_orderbook(orderbook))
        .and_then(handler::get_all_auction_with_details)
}

pub fn get_all_auction_with_details_with_user_participation(
    orderbook: Arc<Orderbook>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_all_auction_with_details_with_user_participation" / H160Wrapper)
        .and(warp::get())
        .and(with_orderbook(orderbook))
        .and_then(handler::get_all_auction_with_details_with_user_participation)
}

pub fn get_signature(
    db: Database,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_signature" / u64 / H160Wrapper)
        .and(warp::get())
        .and(with_signatures(db))
        .and_then(handler::get_signature)
}
pub fn provide_signatures_object(
    orderbook: Arc<Orderbook>,
    db: Database,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("provide_signature")
        .and(warp::post())
        .and(with_orderbook(orderbook))
        .and(with_signatures(db))
        .and(extract_signatures_object_from_json())
        .and_then(handler::provide_signatures)
}

#[cfg(test)]
pub mod test_util {
    use super::*;
    use crate::api::handler::AuctionDetailsForUser;
    use crate::database::SignatureFilter;
    use futures::TryStreamExt;
    use model::auction_details::AuctionDetails;
    use model::signature_object::SignaturePackage;
    use model::signature_object::SignaturesObject;
    use model::user::User;
    use model::Signature;
    use primitive_types::U256;
    use serde_json::json;
    use warp::{http::StatusCode, test::request};

    #[tokio::test]
    async fn get_previous_order_() {
        let orderbook = Orderbook::default();
        let auction_id: u64 = 1;
        let order_1 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 10_u64,
        };
        let order_2 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("3").unwrap(),
            user_id: 10_u64,
        };
        orderbook
            .set_auction_details(auction_id, AuctionDetails::default())
            .await
            .unwrap();
        orderbook.insert_orders(auction_id, vec![order_1]).await;
        let filter = get_previous_order(Arc::new(orderbook));
        let response = request()
            .path(&format!("/get_previous_order/{:}/{:}", auction_id, order_2))
            .method("GET")
            .reply(&filter)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response_order: Order = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(response_order, order_1);
    }

    #[tokio::test]
    async fn get_user_orders_() {
        let orderbook = Orderbook::default();
        let auction_id: u64 = 1;
        let order_1 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 10_u64,
        };
        let order_2 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 9_u64,
        };
        let user = User {
            address: "740a98F8f4fAe0986FB3264Fe4aaCf94ac1EE96f".parse().unwrap(),
            user_id: 10_u64,
        };
        orderbook
            .set_auction_details(auction_id, AuctionDetails::default())
            .await
            .unwrap();
        orderbook
            .insert_orders(auction_id, vec![order_1, order_2])
            .await;
        orderbook.insert_users(vec![user]).await;
        let filter = get_user_orders(Arc::new(orderbook));
        println!(
            "{}",
            format!(
                "/get_user_orders/{:}/{:}",
                auction_id,
                user.show_full_address()
            )
        );
        let response = request()
            .path(&format!(
                "/get_user_orders/{:}/{:}",
                auction_id,
                user.show_full_address()
            ))
            .method("GET")
            .reply(&filter)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response_order: Vec<Order> = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(response_order, vec![order_1]);
    }

    #[tokio::test]
    async fn get_user_orders_without_canceled_or_claimed_() {
        let orderbook = Orderbook::default();
        let auction_id: u64 = 1;
        let order_1 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 10_u64,
        };
        let order_2 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 9_u64,
        };
        let user = User {
            address: "740a98F8f4fAe0986FB3264Fe4aaCf94ac1EE96f".parse().unwrap(),
            user_id: 10_u64,
        };

        orderbook
            .set_auction_details(auction_id, AuctionDetails::default())
            .await
            .unwrap();
        orderbook
            .insert_orders(auction_id, vec![order_1, order_2])
            .await;
        orderbook.insert_users(vec![user]).await;

        let filter = get_user_orders_without_canceled_or_claimed(Arc::new(orderbook));
        println!(
            "{}",
            format!(
                "/get_user_orders_without_canceled_or_claimed/{:}/{:}",
                auction_id,
                user.show_full_address()
            )
        );
        let response = request()
            .path(&format!(
                "/get_user_orders_without_canceled_or_claimed/{:}/{:}",
                auction_id,
                user.show_full_address()
            ))
            .method("GET")
            .reply(&filter)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response_order: Vec<Order> = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(response_order, vec![order_1]);
    }
    #[tokio::test]
    async fn get_auction_details_for_user_() {
        let orderbook = Orderbook::default();
        let auction_id: u64 = 1;
        let order_1 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 10_u64,
        };
        let order_2 = Order {
            sell_amount: U256::from_dec_str("2").unwrap(),
            buy_amount: U256::from_dec_str("2").unwrap(),
            user_id: 9_u64,
        };
        let user = User {
            address: "740a98F8f4fAe0986FB3264Fe4aaCf94ac1EE96f".parse().unwrap(),
            user_id: 10_u64,
        };

        let auction_details = AuctionDetails {
            auction_id,
            ..Default::default()
        };
        orderbook
            .set_auction_details(auction_id, auction_details)
            .await
            .unwrap();
        orderbook
            .insert_orders(auction_id, vec![order_1, order_2])
            .await;
        orderbook.insert_users(vec![user]).await;
        let filter = get_all_auction_with_details_with_user_participation(Arc::new(orderbook));
        let response = request()
            .path(&format!(
                "/get_all_auction_with_details_with_user_participation/{:}",
                user.show_full_address()
            ))
            .method("GET")
            .reply(&filter)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response_details: Vec<AuctionDetailsForUser> =
            serde_json::from_slice(response.body()).unwrap();
        assert_eq!(response_details.get(0).unwrap().has_participation, true);
    }

    #[tokio::test]
    #[ignore]
    async fn get_signature_() {
        let auction_id: u64 = 15;
        let signature = Signature {
            v: 1,
            r: "0200000000000000000000000000000000000000000000000000000000000003"
                .parse()
                .unwrap(),
            s: "0400000000000000000000000000000000000000000000000000000000000005"
                .parse()
                .unwrap(),
        };
        let user = User {
            address: "740a98F8f4fAe0986FB3264Fe4aaCf94ac1EE96f".parse().unwrap(),
            user_id: 10_u64,
        };
        let db = Database::new("postgresql://").unwrap();
        db.clear().await.unwrap();
        db.insert_signatures(
            auction_id,
            vec![SignaturePackage {
                user: user.address,
                signature,
            }],
        )
        .await
        .unwrap();
        let filter = get_signature(db);
        let response = request()
            .path(&format!(
                "/get_signature/{:}/{:}",
                auction_id,
                user.show_full_address()
            ))
            .method("GET")
            .reply(&filter)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response_sig: Signature = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(response_sig, signature);
    }
    #[tokio::test]
    #[ignore]
    async fn provide_new_signatures() {
        let orderbook = Orderbook::default();
        let request_json = json!(
            {"auctionId":10,"chainId":4,"allowListContract":"0x80b8AcA4689EC911F048c4E0976892cCDE14031E","signatures":[{"user":"0x740a98F8f4fAe0986FB3264Fe4aaCf94ac1EE96f","signature":"0x000000000000000000000000000000000000000000000000000000000000001ba38d84751ba93f1b448f137a5755abbd23c22de5b5bcaa05c71b23b79e7423fa5916a4239781aa33c851a9a5c9335dacbc30f9761992597cc9c53f2f39e5ec41"},{"user":"0x04668ec2f57cc15c381b461b9fedab5d451c8f7f","signature":"0x000000000000000000000000000000000000000000000000000000000000001cdb199d09233dc900369c8017332554f3c731eb7519697c44f6b0d73f9545a9f03c64c8ca2f0a1f40a2bb39f2a0941da9909429004f65048d345fe69528cf453c"}]});
        let deserialized_signatures: SignaturesObject =
            serde_json::from_value(request_json.clone()).unwrap();
        orderbook
            .set_auction_details(
                deserialized_signatures.auction_id,
                AuctionDetails {
                    auction_id: deserialized_signatures.auction_id,
                    chain_id: U256::from(deserialized_signatures.chain_id),
                    allow_list_manager: deserialized_signatures.allow_list_contract,
                    allow_list_signer: "0xfB696e9E9e5038DDc78592082689B149AB3a19d5"
                        .parse()
                        .unwrap(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let user = User {
            address: "740a98F8f4fAe0986FB3264Fe4aaCf94ac1EE96f".parse().unwrap(),
            user_id: 10_u64,
        };
        let db = Database::new("postgresql://").unwrap();
        db.clear().await.unwrap();
        let filter = provide_signatures_object(Arc::new(orderbook), db.clone());
        let response = request()
            .path(&"/provide_signature".to_string())
            .method("POST")
            .json(&deserialized_signatures)
            .reply(&filter)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let signature_from_particular_user = db
            .get_signatures(&SignatureFilter {
                auction_id: (deserialized_signatures.auction_id as u32),
                user_address: Some(user.address),
            })
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        assert_eq!(
            deserialized_signatures.signatures[0].signature,
            signature_from_particular_user[0]
        );
    }
}
