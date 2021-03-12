use super::handler;
use crate::api::handler::extract_signatures_object_from_json;
use crate::orderbook::Orderbook;
use crate::signatures::SignatureStore;
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

fn with_signatures(
    signatures: Arc<SignatureStore>,
) -> impl Filter<Extract = (Arc<SignatureStore>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || signatures.clone())
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
    signatures: Arc<SignatureStore>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get_signature" / u64 / H160Wrapper)
        .and(warp::get())
        .and(with_signatures(signatures))
        .and_then(handler::get_signature)
}
pub fn provide_signatures_object(
    orderbook: Arc<Orderbook>,
    signatures: Arc<SignatureStore>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("provide_signature")
        .and(warp::post())
        .and(with_orderbook(orderbook))
        .and(with_signatures(signatures))
        .and(extract_signatures_object_from_json())
        .and_then(handler::provide_signatures)
}

#[cfg(test)]
pub mod test_util {
    use super::*;
    use crate::api::handler::AuctionDetailsForUser;
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
        orderbook
            .insert_orders(auction_id, vec![order_1, order_2])
            .await;
        orderbook.insert_users(vec![user]).await;
        let auction_details = AuctionDetails {
            auction_id,
            ..Default::default()
        };
        orderbook
            .set_auction_details(auction_id, auction_details)
            .await
            .unwrap();
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
    async fn get_signature_() {
        let signature_store = SignatureStore::default();
        let auction_id: u64 = 1;
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
        signature_store
            .insert_signatures(
                auction_id,
                vec![SignaturePackage {
                    user: user.address,
                    signature,
                }],
            )
            .await;
        let filter = get_signature(Arc::new(signature_store));
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
    async fn provide_new_signatures() {
        let signature_store = SignatureStore::default();
        let orderbook = Orderbook::default();
        let request_json = json!(
            {"auctionId":1,"chainId":4,"allowListContract":"0xed52BE1b0071C2f27D10fCc06Ef2e0194cF4E18D","signatures":[{"user":"0x740a98F8f4fAe0986FB3264Fe4aaCf94ac1EE96f","signature":"0x000000000000000000000000000000000000000000000000000000000000001cd5bab0f0dde607f56475301709e2ef5afafef9e59474f572e2321ca05e65a8030acf896a7cff87c470945fd73c8c958c8067dc2c754f72fca7f6038ec2b3bb97"},{"user":"0x04668ec2f57cc15c381b461b9fedab5d451c8f7f","signature":"0x000000000000000000000000000000000000000000000000000000000000001ce63ad8ae9cab71ee08664e9b54c511eb27ad66e2be94ff4199c83d1eff673df2308e604dc8dd4710cef236ccabb16d29cd1830f9c276e570476488cf71e9bebd"}]}); // {"auctionId":1,"chainId":4,"allowListContract":"0xed52BE1b0071C2f27D10fCc06Ef2e0194cF4E18D","signatures":[{"user":"0x740a98F8f4fAe0986FB3264Fe4aaCf94ac1EE96f","signature":"0x000000000000000000000000000000000000000000000000000000000000001b6e5cf2c8aad4817e6fdd674bdfb82ab3aeff34c6a5b26d238f79d8173e7d62714704d23bd2632ff6e1682567ed7fbe943694ec45810d5f87cd93828063fead8a"},{"user":"0x04668ec2f57cc15c381b461b9fedab5d451c8f7f","signature":"0x000000000000000000000000000000000000000000000000000000000000001c0b248f8378c255c3c355b3e4608636f84e26df8e1a8e1975c5ddad85f2f2dacb746ea0232445018cb2b8d174ce333d4b3b06c0e21548a4a77f727dc4051c2e25"}]} );
        let deserialized_signatures: SignaturesObject =
            serde_json::from_value(request_json.clone()).unwrap();
        orderbook
            .set_auction_details(
                deserialized_signatures.auction_id,
                AuctionDetails {
                    auction_id: deserialized_signatures.auction_id,
                    chain_id: U256::from(deserialized_signatures.chain_id),
                    allow_list_manager: deserialized_signatures.allow_list_contract,
                    allow_list_signer: "0x740a98F8f4fAe0986FB3264Fe4aaCf94ac1EE96f"
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
        let signature_store_arc = Arc::new(signature_store);
        let filter = provide_signatures_object(Arc::new(orderbook), signature_store_arc.clone());
        let response = request()
            .path(&"/provide_signature".to_string())
            .method("POST")
            .json(&deserialized_signatures)
            .reply(&filter)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let signature_from_particular_user = signature_store_arc
            .get_signature(deserialized_signatures.auction_id, user.address)
            .await
            .unwrap();
        assert_eq!(
            deserialized_signatures.signatures[0].signature,
            signature_from_particular_user
        );
    }
}
