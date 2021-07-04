use super::Signature;
use crate::DomainSeparator;
use anyhow::Result;
use ethcontract::Address;
use serde::{Deserialize, Serialize};
use web3::{
    signing::{self},
    types::Recovery,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignaturesObject {
    pub auction_id: u64,
    pub chain_id: u64,
    pub allow_list_contract: Address,
    pub signatures: Vec<SignaturePackage>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignaturePackage {
    pub user: Address,
    pub signature: Signature,
}

impl SignaturePackage {
    pub fn validate_signature(
        &self,
        domain_separator: &DomainSeparator,
        user: Address,
        auction_id: u64,
        signer: Address,
    ) -> Result<bool> {
        let v = self.signature.v & 0x1f;
        let message = signing_digest_typed_data(domain_separator, user, auction_id);
        let recovery = Recovery::new(message, v as u64, self.signature.r, self.signature.s);
        if let Some(recovery_data) = recovery.as_signature() {
            return Ok(
                signing::recover(&message, &recovery_data.0, recovery_data.1).unwrap_or_default()
                    == signer,
            );
        }
        Ok(false)
    }
}

// Implements the following ethers.io function
// hardhatRuntime.ethers.utils.defaultAbiCoder.encode(
//     ["bytes32", "address", "uint256"],
//     [
//       hardhatRuntime.ethers.utils._TypedDataEncoder.hashDomain(
//         contractDomain,
//       ),
//       address,
//       taskArgs.auctionId,
//     ],
// plus signMessage, which hashes b"\x19Ethereum Signed Message:\n32"
// and the message hash
fn signing_digest_typed_data(
    domain_separator: &DomainSeparator,
    user: Address,
    auction_id: u64,
) -> [u8; 32] {
    let mut hash_data = [0u8; 96];
    hash_data[0..32].copy_from_slice(&domain_separator.0);
    hash_data[44..64].copy_from_slice(user.as_bytes());
    hash_data[88..96].copy_from_slice(&(auction_id.to_be_bytes()[..]));
    let message_hash = signing::keccak256(&hash_data);
    let mut hash_data = [0u8; 60];
    hash_data[0..28].copy_from_slice(b"\x19Ethereum Signed Message:\n32");
    hash_data[28..60].copy_from_slice(&message_hash);
    signing::keccak256(&hash_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use serde_json::json;

    #[test]
    fn deserialization_and_back() {
        let value = json!(
            {"auctionId":1,"chainId":4,"allowListContract":"0xd45c4a8f7776dfae53b0e03e53b5fcc749a4088d","signatures":[{"user":"0x740a98f8f4fae0986fb3264fe4aacf94ac1ee96f","signature":"0x000000000000000000000000000000000000000000000000000000000000001b772598c8cbf75630449d3edfd4dcddd2eab9e2fc2f854de5f17f58742fa3b55a090a5212d1decfa0c0b43e7466e1b1bb623a3a8ec4ac53adc87b6b905f8676f9"},{"user":"0x04668ec2f57cc15c381b461b9fedab5d451c8f7f","signature":"0x000000000000000000000000000000000000000000000000000000000000001cb2b055fcc1af5e583571280841ca46886f2d83cef1824cdb0b279bfa5e7246572e6f442477c832fcf399d080fe1634b6142e947c34bd62d873bb569e55a8bf66"}]}        );
        let signature_package_1 = SignaturePackage {
            user: "0x740a98f8f4fae0986fb3264fe4aacf94ac1ee96f"
                .parse()
                .unwrap(),
            signature: Signature {
                v: 0x1b,
                r: hex!("772598c8cbf75630449d3edfd4dcddd2eab9e2fc2f854de5f17f58742fa3b55a").into(),
                s: hex!("090a5212d1decfa0c0b43e7466e1b1bb623a3a8ec4ac53adc87b6b905f8676f9").into(),
            },
        };
        let signature_package_2 = SignaturePackage {
            user: "0x04668ec2f57cc15c381b461b9fedab5d451c8f7f"
                .parse()
                .unwrap(),
            signature: Signature {
                v: 0x1c,
                r: hex!("b2b055fcc1af5e583571280841ca46886f2d83cef1824cdb0b279bfa5e724657").into(),
                s: hex!("2e6f442477c832fcf399d080fe1634b6142e947c34bd62d873bb569e55a8bf66").into(),
            },
        };
        let expected_signature_object = SignaturesObject {
            auction_id: 1,
            chain_id: 4,
            allow_list_contract: "0xd45c4a8F7776dFAE53b0E03e53B5fCC749a4088d"
                .parse()
                .unwrap(),
            signatures: vec![signature_package_1, signature_package_2],
        };
        let deserialized: SignaturesObject = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(deserialized, expected_signature_object);
        let serialized = serde_json::to_value(expected_signature_object).unwrap();
        assert_eq!(serialized, value);
    }

    #[test]
    fn validate_input() {
        let signer: Address = "0x740a98F8f4fAe0986FB3264Fe4aaCf94ac1EE96f"
            .parse()
            .unwrap();
        let value = json!(
            {"auctionId":1,"chainId":4,"allowListContract":"0xed52BE1b0071C2f27D10fCc06Ef2e0194cF4E18D","signatures":[{"user":"0x740a98F8f4fAe0986FB3264Fe4aaCf94ac1EE96f","signature":"0x000000000000000000000000000000000000000000000000000000000000001cd5bab0f0dde607f56475301709e2ef5afafef9e59474f572e2321ca05e65a8030acf896a7cff87c470945fd73c8c958c8067dc2c754f72fca7f6038ec2b3bb97"},{"user":"0x04668ec2f57cc15c381b461b9fedab5d451c8f7f","signature":"0x000000000000000000000000000000000000000000000000000000000000001ce63ad8ae9cab71ee08664e9b54c511eb27ad66e2be94ff4199c83d1eff673df2308e604dc8dd4710cef236ccabb16d29cd1830f9c276e570476488cf71e9bebd"}]}); // {"auctionId":1,"chainId":4,"allowListContract":"0xed52BE1b0071C2f27D10fCc06Ef2e0194cF4E18D","signatures":[{"user":"0x740a98F8f4fAe0986FB3264Fe4aaCf94ac1EE96f","signature":"0x000000000000000000000000000000000000000000000000000000000000001b6e5cf2c8aad4817e6fdd674bdfb82ab3aeff34c6a5b26d238f79d8173e7d62714704d23bd2632ff6e1682567ed7fbe943694ec45810d5f87cd93828063fead8a"},{"user":"0x04668ec2f57cc15c381b461b9fedab5d451c8f7f","signature":"0x000000000000000000000000000000000000000000000000000000000000001c0b248f8378c255c3c355b3e4608636f84e26df8e1a8e1975c5ddad85f2f2dacb746ea0232445018cb2b8d174ce333d4b3b06c0e21548a4a77f727dc4051c2e25"}]} );
        let deserialized_signatures: SignaturesObject = serde_json::from_value(value).unwrap();
        let domain_separator_rinkeby = DomainSeparator::get_domain_separator(
            deserialized_signatures.chain_id,
            deserialized_signatures.allow_list_contract,
        );
        for signature_pair in deserialized_signatures.signatures {
            assert!(signature_pair
                .validate_signature(
                    &domain_separator_rinkeby,
                    signature_pair.user,
                    deserialized_signatures.auction_id,
                    signer
                )
                .unwrap())
        }
    }
}
