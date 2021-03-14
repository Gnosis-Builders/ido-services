use super::*;
use anyhow::{anyhow, Context, Result};
use ethcontract::Address;
use futures::{stream::TryStreamExt, Stream};
use model::Signature;
use primitive_types::H160;
use std::convert::TryInto;

/// Any default value means that this field is unfiltered.
#[derive(Default)]
pub struct SignatureFilter {
    pub auction_id: u32,
    pub user_address: Option<H160>,
}

impl Database {
    pub async fn insert_signature(
        &self,
        auction_id: u32,
        user_address: Address,
        signature: &Signature,
    ) -> Result<()> {
        const QUERY: &str = "\
            INSERT INTO signatures (
                auction_id, user_address, signature) \
            VALUES ( \
                $1, $2, $3);";
        sqlx::query(QUERY)
            .bind(auction_id)
            .bind(user_address.as_bytes())
            .bind(signature.to_bytes().as_ref())
            .execute(&self.pool)
            .await
            .context("insert_signature failed")
            .map(|_| ())
    }

    pub fn signatures<'a>(
        &'a self,
        filter: &'a SignatureFilter,
    ) -> impl Stream<Item = Result<Signature>> + 'a {
        const QUERY: &str = "\
        SELECT \
            s.signature \
        FROM \
            signatures s 
        WHERE \
            s.auction_id = $1 AND \
            ($2 IS NULL OR s.user_address = $2) 
         ";
        sqlx::query_as(QUERY)
            .bind(filter.auction_id)
            .bind(filter.user_address.as_ref().map(|h160| h160.as_bytes()))
            .fetch(&self.pool)
            .err_into()
            .and_then(|row: SignaturesQueryRow| async move { row.into_signature() })
    }
}
#[derive(sqlx::FromRow, Debug)]
struct SignaturesQueryRow {
    signature: Vec<u8>,
}

impl SignaturesQueryRow {
    fn into_signature(self) -> Result<Signature> {
        Ok(Signature::from_bytes(
            &self
                .signature
                .try_into()
                .map_err(|_| anyhow!("signature has wrong length"))?,
        ))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use futures::StreamExt;
    use std::str::FromStr;

    #[tokio::test]
    #[ignore]
    async fn postgres_insert_same_signature_twice_fails() {
        let db = Database::new("postgresql://").unwrap();
        db.clear().await.unwrap();
        let auction_id = 1u64;
        let user_address = H160::zero();
        let signature = Signature::default();
        db.insert_signature(auction_id as u32, user_address, &signature)
            .await
            .unwrap();
        assert!(db
            .insert_signature(auction_id as u32, user_address, &signature)
            .await
            .is_err());
    }

    #[tokio::test]
    #[ignore]
    async fn postgres_signature_roundtrip() {
        let db = Database::new("postgresql://").unwrap();
        db.clear().await.unwrap();
        let auction_id = 2;
        let user_address = H160::zero();
        let filter = SignatureFilter {
            auction_id,
            user_address: Some(user_address),
        };
        assert!(db.signatures(&filter).boxed().next().await.is_none());
        let value = String::from("0x000000000000000000000000000000000000000000000000000000000000001b772598c8cbf75630449d3edfd4dcddd2eab9e2fc2f854de5f17f58742fa3b55a090a5212d1decfa0c0b43e7466e1b1bb623a3a8ec4ac53adc87b6b905f8676f9");
        let signature = Signature::from_str(&value).unwrap();

        db.insert_signature(auction_id as u32, user_address, &signature)
            .await
            .unwrap();
        assert_eq!(
            db.signatures(&filter)
                .try_collect::<Vec<Signature>>()
                .await
                .unwrap(),
            vec![signature]
        );
    }
    #[tokio::test]
    #[ignore]
    async fn postgres_get_all_signatures_roundtrip() {
        let db = Database::new("postgresql://").unwrap();
        db.clear().await.unwrap();
        let auction_id = 3;
        let filter = SignatureFilter {
            auction_id,
            user_address: None,
        };
        let user_address = H160::zero();
        let user_address_2 = "0x04668ec2f57cc15c381b461b9fedab5d451c8f7f"
            .parse()
            .unwrap();
        let value = String::from("0x000000000000000000000000000000000000000000000000000000000000001b772598c8cbf75630449d3edfd4dcddd2eab9e2fc2f854de5f17f58742fa3b55a090a5212d1decfa0c0b43e7466e1b1bb623a3a8ec4ac53adc87b6b905f8676f9");
        let signature_1 = Signature::from_str(&value).unwrap();
        let value = String::from("0x000000000000000000000000000000000000000000000000000000000000001b172598c8cbf75630449d3edfd4dcddd2eab9e2fc2f854de5f17f58742fa3b55a090a5212d1decfa0c0b43e7466e1b1bb623a3a8ec4ac53adc87b6b905f8676f9");
        let signature_2 = Signature::from_str(&value).unwrap();

        db.insert_signature(auction_id as u32, user_address, &signature_1)
            .await
            .unwrap();
        db.insert_signature(auction_id as u32, user_address_2, &signature_2)
            .await
            .unwrap();
        assert_eq!(
            db.signatures(&filter)
                .try_collect::<Vec<Signature>>()
                .await
                .unwrap(),
            vec![signature_1, signature_2]
        );
    }
}
