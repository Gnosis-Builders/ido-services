use ethcontract::Address;
use model::signature_object::SignaturePackage;
use model::Signature;
use primitive_types::H160;
use std::collections::{hash_map::Entry, HashMap};
use tokio::sync::RwLock;

#[derive(Default, Debug)]
pub struct SignatureStore {
    pub signatures: RwLock<HashMap<u64, HashMap<Address, Signature>>>,
}

impl SignatureStore {
    #[allow(dead_code)]
    pub fn new() -> Self {
        SignatureStore {
            signatures: RwLock::new(HashMap::new()),
        }
    }
    pub async fn get_signature(&self, auction_id: u64, user: H160) -> Option<Signature> {
        let hashmap = self.signatures.read().await;
        if let Some(sig_hashmap) = hashmap.get(&auction_id) {
            if let Some(signature) = sig_hashmap.get(&user) {
                return Some(*signature);
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    pub async fn insert_signatures(
        &self,
        auction_id: u64,
        users_and_signatures: Vec<SignaturePackage>,
    ) {
        if users_and_signatures.is_empty() {
            return;
        }
        {
            let mut hashmap = self.signatures.write().await;
            match hashmap.entry(auction_id) {
                Entry::Occupied(mut sig_hashmap) => {
                    for signature_pair in users_and_signatures {
                        sig_hashmap
                            .get_mut()
                            .insert(signature_pair.user, signature_pair.signature);
                    }
                }
                Entry::Vacant(_) => {
                    let mut new_hashmap = HashMap::new();
                    for signature_pair in users_and_signatures {
                        new_hashmap.insert(signature_pair.user, signature_pair.signature);
                    }
                    hashmap.insert(auction_id, new_hashmap);
                }
            }
        }
    }
}

#[cfg(test)]
pub mod test_util {
    use super::*;
    use model::signature_object::SignaturePackage;
    use model::user::User;
    use model::Signature;

    #[tokio::test]
    async fn insert_and_get_signature_() {
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
        let received_signature = signature_store
            .get_signature(auction_id, user.address)
            .await
            .unwrap();
        assert_eq!(received_signature, signature)
    }
}
