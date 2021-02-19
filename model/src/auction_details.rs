use super::order::PricePoint;
use ethcontract::Address;
use primitive_types::U256;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::cmp::PartialOrd;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionDetails {
    pub auction_id: u64,
    pub order: PricePoint,
    pub symbol_auctioning_token: String,
    pub symbol_bidding_token: String,
    pub address_auctioning_token: Address,
    pub address_bidding_token: Address,
    pub decimals_auctioning_token: U256,
    pub decimals_bidding_token: U256,
    pub end_time_timestamp: u64,
    pub starting_timestamp: u64,
    pub current_clearing_price: f64,
    pub is_private_auction: bool,
}

impl AuctionDetails {
    fn bidding_volume(&self) -> f64 {
        self.order.volume * self.order.price
    }
}

// Auction details are sortable by their interest
// the higher the min bidding token amount is,
// the more interesting an auction should be.

impl PartialEq for AuctionDetails {
    fn eq(&self, other: &Self) -> bool {
        float_cmp::approx_eq!(f64, self.bidding_volume(), other.bidding_volume(), ulps = 2)
    }
}
impl Eq for AuctionDetails {}

impl PartialOrd for AuctionDetails {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AuctionDetails {
    fn cmp(&self, other: &Self) -> Ordering {
        if float_cmp::approx_eq!(f64, self.bidding_volume(), other.bidding_volume(), ulps = 2) {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}
