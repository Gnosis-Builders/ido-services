use lazy_static::lazy_static;
use primitive_types::U256;
use serde::Serialize;
use serde::Serializer;
use serde::{de, Deserialize, Deserializer};
use std::cmp::Ordering;
use std::convert::TryInto;
use std::fmt::{self, Display};
use std::str::FromStr;

#[derive(Eq, PartialEq, Clone, Debug, Copy, Default)]
pub struct Order {
    pub sell_amount: U256,
    pub buy_amount: U256,
    pub user_id: u64,
}

#[derive(Eq, PartialEq, Clone, Debug, Copy, Default)]
pub struct OrderWithAuctionID {
    pub auction_id: u64,
    pub order: Order,
}

#[derive(Default, Debug, Serialize)]
pub struct OrderbookDisplay {
    pub asks: Vec<PricePoint>,
    pub bids: Vec<PricePoint>,
}
#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PricePoint {
    pub price: f64,
    pub volume: f64,
}
impl PricePoint {
    pub fn invert_price(&self) -> Self {
        PricePoint {
            price: 1_f64 / self.price,
            volume: self.volume,
        }
    }
}
lazy_static! {
    pub static ref TEN: U256 = U256::from_dec_str("10").unwrap();
    pub static ref EIGHTEEN: U256 = U256::from_dec_str("18").unwrap();
}
impl PartialEq for PricePoint {
    fn eq(&self, other: &Self) -> bool {
        float_cmp::approx_eq!(f64, self.volume, other.volume, ulps = 2)
            && float_cmp::approx_eq!(f64, self.price, other.price, ulps = 2)
    }
}
impl Eq for PricePoint {}
impl Order {
    pub fn to_price_point(
        &self,
        decimals_buy_token: U256,
        decimals_sell_token: U256,
    ) -> PricePoint {
        let mut price_denominator = self
            .buy_amount
            .checked_mul(TEN.pow(decimals_sell_token))
            .expect("buy_amount should not overflow")
            .to_f64_lossy();
        // avoid special case where we would divide by zero
        if price_denominator == 0_f64 {
            price_denominator = 1_f64;
        }
        let price_numerator = self
            .sell_amount
            .checked_mul(TEN.pow(decimals_buy_token))
            .expect("sell_amount should not overflow")
            .to_f64_lossy();
        let volume_numerator = self.sell_amount.to_f64_lossy();
        let volume_denominator = (TEN.pow(decimals_sell_token)).to_f64_lossy();
        PricePoint {
            price: price_numerator / price_denominator,
            volume: volume_numerator / volume_denominator,
        }
    }
}

impl FromStr for Order {
    type Err = hex::FromHexError;
    fn from_str(s: &str) -> Result<Order, hex::FromHexError> {
        let s_without_prefix = s.strip_prefix("0x").unwrap_or(s);
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(s_without_prefix, &mut bytes)?;
        // let sell_amount_bytes: [u8; 32] = bytes[20..32].try_into().expect("slice with incorrect length");
        // let buy_amount_bytes: [u8;32] = .try_into().expect("slice with incorrect length");
        Ok(Order {
            sell_amount: U256::from_big_endian(&bytes[20..32]),
            buy_amount: U256::from_big_endian(&bytes[8..20]),
            user_id: u64::from_be_bytes(bytes[..8].try_into().expect("conversion not possible")),
        })
    }
}

impl Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut bytes = [0u8; 2 + 64];
        bytes[..2].copy_from_slice(b"0x");
        hex::encode_to_slice(self.user_id.to_be_bytes(), &mut bytes[2..18]).unwrap();
        // Can only fail if the buffer size does not match but we know it is correct.
        let mut interim_bytes = [0u8; 32];
        self.buy_amount.to_big_endian(&mut interim_bytes);
        let b: [u8; 12] = interim_bytes[20..32]
            .try_into()
            .expect("slice with incorrect length");
        hex::encode_to_slice(b, &mut bytes[18..42]).unwrap();
        self.sell_amount.to_big_endian(&mut interim_bytes);
        let b: [u8; 12] = interim_bytes[20..32]
            .try_into()
            .expect("slice with incorrect length");
        hex::encode_to_slice(b, &mut bytes[42..66]).unwrap();
        // Hex encoding is always valid utf8.
        let str = std::str::from_utf8(&bytes).unwrap();
        f.write_str(str)
    }
}

impl PartialOrd for Order {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Order {
    fn cmp(&self, other: &Self) -> Ordering {
        if self
            .buy_amount
            .checked_mul(other.sell_amount)
            .lt(&self.sell_amount.checked_mul(other.buy_amount))
        {
            return Ordering::Less;
        }
        if self
            .buy_amount
            .checked_mul(other.sell_amount)
            .gt(&self.sell_amount.checked_mul(other.buy_amount))
        {
            return Ordering::Greater;
        }
        self.user_id.partial_cmp(&other.user_id).unwrap()
    }
}

impl Serialize for Order {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for Order {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(de::Error::custom)
    }
}

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn display_and_from_str() {
        let order = Order {
            sell_amount: U256::from_dec_str("1230").unwrap(),
            buy_amount: U256::from_dec_str("123").unwrap(),
            user_id: 10_u64,
        };
        let expected = "0x000000000000000a00000000000000000000007b0000000000000000000004ce";
        assert_eq!(order.to_string(), expected);
        assert_eq!(format!("{}", order), expected);
        let deserialized: Order = Order::from_str(expected).unwrap();
        assert_eq!(deserialized, order);
    }

    #[test]
    fn ordering_of_orders() {
        let normal_order = Order {
            sell_amount: U256::from_dec_str("1230").unwrap(),
            buy_amount: U256::from_dec_str("123").unwrap(),
            user_id: 10_u64,
        };
        let higher_priced_order = Order {
            sell_amount: U256::from_dec_str("1230").unwrap(),
            buy_amount: U256::from_dec_str("1000").unwrap(),
            user_id: 10_u64,
        };
        assert_eq!(normal_order.cmp(&higher_priced_order), Ordering::Less);
        assert_eq!(normal_order.cmp(&normal_order), Ordering::Equal);
        assert_eq!(higher_priced_order.cmp(&normal_order), Ordering::Greater);
    }

    #[test]
    fn to_price_point_with_18_digits() {
        let normal_order = Order {
            sell_amount: U256::from_dec_str("100000000000000000000").unwrap(),
            buy_amount: U256::from_dec_str("110000000000000000000").unwrap(),
            user_id: 10_u64,
        };
        let expected_price_point = PricePoint {
            price: 10_f64 / 11_f64,
            volume: 100.0_f64,
        };
        assert_eq!(
            normal_order.to_price_point(*EIGHTEEN, *EIGHTEEN),
            expected_price_point
        );
    }
    #[test]
    fn to_price_point_without_18_digits() {
        let normal_order = Order {
            sell_amount: U256::from_dec_str("100000000000000000000").unwrap(),
            buy_amount: U256::from_dec_str("110000000000000000000").unwrap(),
            user_id: 10_u64,
        };
        let expected_price_point = PricePoint {
            price: 10_f64 / (11_f64 * 10_f64.powi(12)),
            volume: 100.0_f64,
        };
        assert_eq!(
            normal_order.to_price_point(U256::from("6"), *EIGHTEEN),
            expected_price_point
        );
    }
}
