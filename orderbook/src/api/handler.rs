use crate::api::filter::H160Wrapper;
use crate::database::Database;
use crate::database::SignatureFilter;
use crate::health::HttpHealthEndpoint;
use crate::orderbook::Orderbook;
use futures::future::join_all;
use futures::TryStreamExt;
use model::auction_details::AuctionDetails;
use model::order::Order;
use model::signature_object::SignaturesObject;
use model::DomainSeparator;
use model::Signature;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc};
use warp::Filter;
use warp::Rejection;
use warp::{
    http::StatusCode,
    reply::{json, with_status},
};

const MAX_JSON_BODY_PAYLOAD: u64 = 1024 * 10; // rejecting more than 10kbits uploads

pub fn extract_signatures_object_from_json(
) -> impl Filter<Extract = (SignaturesObject,), Error = Rejection> + Clone {
    // (rejecting huge payloads)...
    warp::body::content_length_limit(MAX_JSON_BODY_PAYLOAD).and(warp::body::json())
}

pub async fn readiness(health: Arc<HttpHealthEndpoint>) -> Result<impl warp::Reply, Infallible> {
    if health.is_ready() {
        Ok(with_status(json(&""), StatusCode::NO_CONTENT))
    } else {
        Ok(with_status(
            json(&"service unavailable"),
            StatusCode::SERVICE_UNAVAILABLE,
        ))
    }
}

pub async fn get_signature(
    auction_id: u64,
    user: H160Wrapper,
    db: Database,
) -> Result<impl warp::Reply, Infallible> {
    if let Ok(signature) = db
        .get_signatures(&SignatureFilter {
            auction_id: (auction_id as u32),
            user_address: Some(user.0),
        })
        .try_collect::<Vec<Signature>>()
        .await
    {
        if signature.len() != 1 {
            return Ok(with_status(
                json(&format!("Signature not available for user {:}", user.0)),
                StatusCode::OK,
            ));
        }
        Ok(with_status(json(&signature[0]), StatusCode::OK))
    } else {
        Ok(with_status(
            json(&format!(
                "Could not retrieve signature for user {:}",
                user.0
            )),
            StatusCode::BAD_REQUEST,
        ))
    }
}
pub async fn provide_signatures(
    orderbook: Arc<Orderbook>,
    db: Database,
    signature_object: SignaturesObject,
) -> Result<impl warp::Reply, Infallible> {
    let event_details;
    let event_details_obj = orderbook
        .get_auction_with_details(signature_object.auction_id)
        .await;
    if let Err(err) = &event_details_obj {
        return Ok(with_status(
            json(&format!("Internal error: {:?}", err)),
            StatusCode::BAD_REQUEST,
        ));
    } else {
        event_details = event_details_obj.unwrap();
    }
    if event_details.chain_id.as_u64() != signature_object.chain_id {
        return Ok(with_status(
            json(&format!(
                "Wrong chain id. This API talks to the chain id {:?}",
                event_details.chain_id.as_u64()
            )),
            StatusCode::BAD_REQUEST,
        ));
    }
    if event_details.allow_list_manager != signature_object.allow_list_contract {
        return Ok(with_status(
            json(&format!(
                "Wrong allow list contract used. Auction is scheduled with {:?}",
                event_details.allow_list_manager
            )),
            StatusCode::BAD_REQUEST,
        ));
    }
    let domain_separator_of_call = DomainSeparator::get_domain_separator(
        signature_object.chain_id,
        signature_object.allow_list_contract,
    );
    let future_results = join_all(signature_object.signatures.iter().map(|signature_pair| {
        let allow_list_signer = event_details.allow_list_signer;
        let auction_id = signature_object.auction_id;
        async move {
            (
                signature_pair.clone(),
                signature_pair.validate_signature(
                    &domain_separator_of_call,
                    signature_pair.user,
                    auction_id,
                    allow_list_signer,
                ),
            )
        }
    }))
    .await;

    for (signature_pair, signature_ok) in future_results {
        if let Err(err) = signature_ok {
            return Ok(with_status(
                json(&format!(
                    "Error {:?} while decoding signature {:?} for user {:?} ",
                    err, signature_pair.signature, signature_pair.user
                )),
                StatusCode::BAD_REQUEST,
            ));
        } else if !signature_ok.unwrap() {
            return Ok(with_status(
                json(&format!(
                    "Signature {:?} for user {:?} is not valid",
                    signature_pair.signature, signature_pair.user
                )),
                StatusCode::BAD_REQUEST,
            ));
        }
    }
    let insert_results = db
        .insert_signatures(signature_object.auction_id, signature_object.signatures)
        .await;
    if let Err(error) = insert_results {
        return Ok(with_status(
            json(&format!(
                "Errors: {:?} while inserting data into database ",
                error
            )),
            StatusCode::BAD_REQUEST,
        ));
    }
    Ok(with_status(
        json(&"All signatures added".to_string()),
        StatusCode::OK,
    ))
}

pub async fn get_previous_order(
    auction_id: u64,
    order: Order,
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let order = orderbook.get_previous_order(auction_id, order).await;
    Ok(with_status(json(&order), StatusCode::OK))
}

pub async fn get_user_orders(
    auction_id: u64,
    user: H160Wrapper,
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let order = orderbook.get_user_orders(auction_id, user.0).await;
    Ok(with_status(json(&order), StatusCode::OK))
}

pub async fn get_user_orders_without_canceled_or_claimed(
    auction_id: u64,
    user: H160Wrapper,
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let order = orderbook
        .get_user_orders_without_canceled_claimed(auction_id, user.0)
        .await;
    Ok(with_status(json(&order), StatusCode::OK))
}

pub async fn get_clearing_order_and_volume(
    auction_id: u64,
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let order = orderbook.get_clearing_order_and_volume(auction_id).await;
    match order {
        Ok(order) => Ok(with_status(json(&order), StatusCode::OK)),
        Err(err) => Ok(with_status(
            json(&format!(
                "Errors: {:?} while calculating the clearing price ",
                err
            )),
            StatusCode::BAD_REQUEST,
        )),
    }
}

pub async fn get_order_book_display_data(
    auction_id: u64,
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let orderbook_data = orderbook.get_order_book_display(auction_id).await;
    match orderbook_data {
        Err(err) => Ok(with_status(
            json(&format!("{:}", err)),
            StatusCode::BAD_REQUEST,
        )),
        Ok(orderbook_data) => Ok(with_status(json(&orderbook_data), StatusCode::OK)),
    }
}

pub async fn get_details_of_most_interesting_closed_auctions(
    number_of_auctions: u64,
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let auction_detail_data = orderbook
        .get_most_interesting_closed_auctions(number_of_auctions)
        .await;
    match auction_detail_data {
        Err(err) => Ok(with_status(
            json(&format!("{:}", err)),
            StatusCode::BAD_REQUEST,
        )),
        Ok(auction_detail_data) => Ok(with_status(json(&auction_detail_data), StatusCode::OK)),
    }
}

pub async fn get_details_of_most_interesting_auctions(
    number_of_auctions: u64,
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let auction_detail_data = orderbook
        .get_most_interesting_auctions(number_of_auctions)
        .await;
    match auction_detail_data {
        Err(err) => Ok(with_status(
            json(&format!("{:}", err)),
            StatusCode::BAD_REQUEST,
        )),
        Ok(auction_detail_data) => Ok(with_status(json(&auction_detail_data), StatusCode::OK)),
    }
}

pub async fn get_all_auction_with_details(
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let auction_detail_data = orderbook.get_all_auction_with_details().await;
    match auction_detail_data {
        Err(err) => Ok(with_status(
            json(&format!("{:}", err)),
            StatusCode::BAD_REQUEST,
        )),
        Ok(auction_detail_data) => Ok(with_status(json(&auction_detail_data), StatusCode::OK)),
    }
}
pub async fn get_auction_with_details(
    auction_id: u64,
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let auction_detail_data = orderbook.get_auction_with_details(auction_id).await;
    match auction_detail_data {
        Err(err) => Ok(with_status(
            json(&format!("{:}", err)),
            StatusCode::BAD_REQUEST,
        )),
        Ok(auction_detail_data) => Ok(with_status(json(&auction_detail_data), StatusCode::OK)),
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionDetailsForUser {
    pub has_participation: bool,
    #[serde(flatten)]
    pub auction_details: AuctionDetails,
}
pub async fn get_all_auction_with_details_with_user_participation(
    user_address: H160Wrapper,
    orderbook: Arc<Orderbook>,
) -> Result<impl warp::Reply, Infallible> {
    let auction_detail_request = orderbook.get_all_auction_with_details().await;
    let auction_detail = match auction_detail_request {
        Ok(data) => data,
        Err(err) => {
            return Ok(with_status(
                json(&format!("{:}", err)),
                StatusCode::BAD_REQUEST,
            ))
        }
    };
    let user_id_request = orderbook.get_user_id(user_address.0).await;
    let user_id = match user_id_request {
        Ok(data) => data,
        Err(err) => {
            return Ok(with_status(
                json(&format!("{:}", err)),
                StatusCode::BAD_REQUEST,
            ))
        }
    };
    let auction_ids_with_participation = orderbook.get_used_auctions(user_id).await;
    let auction_details_for_user: Vec<AuctionDetailsForUser> = auction_detail
        .iter()
        .map(|auction_detail| AuctionDetailsForUser {
            has_participation: auction_ids_with_participation.contains(&auction_detail.auction_id),
            auction_details: auction_detail.clone(),
        })
        .collect();
    Ok(with_status(json(&auction_details_for_user), StatusCode::OK))
}
