//! Module containing The Graph API client used for retrieving Balancer weighted
//! pools from the Balancer V2 subgraph.
//!
//! The pools retrieved from this client are used to prime the graph event store
//! to reduce start-up time. We do not use this in general for retrieving pools
//! as to:
//! - not rely on external services
//! - ensure that we are using the latest up-to-date pool data by using events
//!   from the node

use super::thegraph::SubgraphClient;
use anyhow::anyhow;
use anyhow::{bail, Result};
use serde_json::json;
use std::collections::HashMap;

#[macro_export]
macro_rules! json_map {
    ($($key:expr => $value:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut map = ::serde_json::Map::<String, ::serde_json::Value>::new();
        $(
            map.insert(($key).into(), ($value).into());
        )*
        map
    }}
}
/// A client to the Uniswap V2 subgraph.
///
/// This client is not implemented to allow general GraphQL queries, but instead
/// implements high-level methods that perform GraphQL queries under the hood.
pub struct UniswapSubgraphClient {
    client: SubgraphClient,
    request_store: HashMap<u64, f64>,
}

impl UniswapSubgraphClient {
    /// Creates a new Balancer subgraph client for the specified chain ID.
    pub fn for_chain(chain_id: u64) -> Result<Self> {
        let subgraph_name = match chain_id {
            1 => "uniswap-v2",
            4 => "uniswap-rinkeby-v2",
            _ => bail!("unsupported chain {}", chain_id),
        };
        Ok(Self {
            client: SubgraphClient::new("uniswap", subgraph_name)?,
            request_store: HashMap::new(),
        })
    }

    /// Retrieves the list of registered pools from the subgraph.
    pub async fn get_eth_usd_price(&mut self, timestamp: u64) -> Result<f64> {
        let sec_per_day = (24 * 60 * 60) as u64;
        let div = timestamp / sec_per_day;
        let timestamp_of_day = div * sec_per_day;
        if let Some(price) = self.request_store.get(&timestamp_of_day) {
            Ok(*price)
        } else {
            let amm = self
                .client
                .query::<price_query::Data>(
                    price_query::QUERY,
                    Some(json_map! {
                        "date" => json!(timestamp_of_day),
                    }),
                )
                .await?
                .pair_day_datas;
            if let Some(amm_ratio) = amm.get(0) {
                let price = amm_ratio.reserve0 / amm_ratio.reserve1;
                self.request_store.insert(timestamp_of_day, price);
                Ok(price)
            } else {
                Err(anyhow!(
                    "Missing attribute entry in the api response for the price"
                ))
            }
        }
    }
}

mod price_query {

    use anyhow::Result;
    use serde::de::{Deserializer, Error as _};
    use serde::Deserialize;

    use std::borrow::Cow;

    pub const QUERY: &str = r#"
        query PairDayData($date: Int) {
            pairDayDatas(
                first: 1,
                where: { date: $date, pairAddress: "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc" }
            ) {
                reserve0
                reserve1
              }
        }
    "#;

    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Data {
        pub pair_day_datas: Vec<AmmRatio>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct AmmRatio {
        #[serde(deserialize_with = "deserialize_decimal_f64")]
        pub reserve0: f64,
        #[serde(deserialize_with = "deserialize_decimal_f64")]
        pub reserve1: f64,
    }
    pub fn deserialize_decimal_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let decimal_str = Cow::<str>::deserialize(deserializer)?;
        decimal_str.parse::<f64>().map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_amm_data() {
        use price_query::*;

        let data = Data {
            pair_day_datas: vec![AmmRatio {
                reserve0: 156_588_648.020_372_78_f64,
                reserve1: 1.430_946_749_699_242_f64,
            }],
        };
        assert_eq!(
            serde_json::from_value::<Data>(json!(
                {
                      "pairDayDatas": [
                        {
                          "reserve0": "156588648.02037278",
                          "reserve1": "1.430946749699242"
                        }
                      ]
                  }
            ))
            .unwrap(),
            data
        );
    }

    #[tokio::test(flavor = "current_thread")]
    #[ignore]
    async fn uniswap_price_subgraph_query() {
        let mut client = UniswapSubgraphClient::for_chain(1).unwrap();
        let timestamp = 1625260016u64;
        let response = client.get_eth_usd_price(timestamp).await;
        println!("{:?}", response);
    }
}
