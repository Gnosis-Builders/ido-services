pub mod auction_details;
pub mod order;
pub mod signature_object;
pub mod user;

use ethabi::{encode, Token};
use hex::{FromHex, FromHexError};
use lazy_static::lazy_static;
use primitive_types::{H160, H256};
use serde::{de, Deserialize, Serialize};
use serde::{Deserializer, Serializer};
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use web3::signing;

#[derive(Eq, PartialEq, Clone, Copy, Debug, Default, Hash)]
pub struct Signature {
    pub r: H256,
    pub s: H256,
    pub v: u8,
}

impl Signature {
    /// v + r + s
    pub fn convert_to_bytes(&self) -> [u8; 65] {
        let mut bytes = [0u8; 65];
        bytes[0] = self.v;
        bytes[1..33].copy_from_slice(self.r.as_bytes());
        bytes[33..65].copy_from_slice(self.s.as_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8; 65]) -> Self {
        Signature {
            r: H256::from_slice(&bytes[1..33]),
            s: H256::from_slice(&bytes[33..65]),
            v: bytes[0],
        }
    }
}

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl FromStr for Signature {
    type Err = hex::FromHexError;
    fn from_str(s: &str) -> Result<Signature, hex::FromHexError> {
        let mut s = s.strip_prefix("0x").unwrap_or(s);
        s = s
            .strip_prefix("00000000000000000000000000000000000000000000000000000000000000")
            .unwrap_or(s);
        let mut bytes = [0u8; 65];
        hex::decode_to_slice(s, &mut bytes)?;
        Ok(Signature::from_bytes(&bytes))
    }
}

impl Display for Signature {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut bytes = [0u8; 65 * 2];
        // Can only fail if the buffer size does not match but we know it is correct.
        hex::encode_to_slice(&self.convert_to_bytes(), &mut bytes).unwrap();
        // Hex encoding is always valid utf8.
        let str = std::str::from_utf8(&bytes).unwrap();
        let mut full_str =
            String::from("0x00000000000000000000000000000000000000000000000000000000000000");
        full_str.push_str(str);
        f.write_str(&full_str)
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(de::Error::custom)
    }
}

#[derive(Copy, Eq, PartialEq, Clone, Default)]
pub struct DomainSeparator(pub [u8; 32]);

impl FromStr for DomainSeparator {
    type Err = FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(FromHex::from_hex(s)?))
    }
}

impl Debug for DomainSeparator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut hex = [0u8; 64];
        // Unwrap because we know the length is correct.
        hex::encode_to_slice(self.0, &mut hex).unwrap();
        // Unwrap because we know it is valid utf8.
        f.write_str(std::str::from_utf8(&hex).unwrap())
    }
}

impl DomainSeparator {
    pub fn get_domain_separator(chain_id: u64, contract_address: H160) -> Self {
        lazy_static! {
            /// The EIP-712 domain name used for computing the domain separator.
            static ref DOMAIN_NAME: [u8; 32] = signing::keccak256(b"AccessManager");

            /// The EIP-712 domain version used for computing the domain separator.
            static ref DOMAIN_VERSION: [u8; 32] = signing::keccak256(b"v1");

            /// The EIP-712 domain type used computing the domain separator.
            static ref DOMAIN_TYPE_HASH: [u8; 32] = signing::keccak256(
                b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
            );
        }
        let abi_encode_string = encode(&[
            Token::Uint((*DOMAIN_TYPE_HASH).into()),
            Token::Uint((*DOMAIN_NAME).into()),
            Token::Uint((*DOMAIN_VERSION).into()),
            Token::Uint(chain_id.into()),
            Token::Address(contract_address),
        ]);

        DomainSeparator(signing::keccak256(abi_encode_string.as_slice()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn domain_separator_rinkeby() {
        let contract_address: H160 = hex!("ed52BE1b0071C2f27D10fCc06Ef2e0194cF4E18D").into();
        let chain_id: u64 = 4;
        let domain_separator_rinkeby =
            DomainSeparator::get_domain_separator(chain_id, contract_address);
        // domain separator is taken from rinkeby deployment at address 91D6387ffbB74621625F39200d91a50386C9Ab15
        let expected_domain_separator = DomainSeparator(hex!(
            "ef81736805e079cbd2261d46a8b80d018644aa78b3bc9ae635f0c4baf0fa6c90"
        ));
        assert_eq!(domain_separator_rinkeby, expected_domain_separator);
    }

    #[test]
    fn domain_separator_debug() {
        let expected_domain_separator = DomainSeparator(hex!(
            "ef81736805e079cbd2261d46a8b80d018644aa78b3bc9ae635f0c4baf0fa6c90"
        ));
        let domain_separator_from_str = DomainSeparator::from_str(
            "ef81736805e079cbd2261d46a8b80d018644aa78b3bc9ae635f0c4baf0fa6c90",
        )
        .unwrap();
        assert_eq!(domain_separator_from_str, expected_domain_separator);
    }

    #[test]
    fn serialize_and_deserialize_signature() {
        let value: String = String::from("0x000000000000000000000000000000000000000000000000000000000000001b772598c8cbf75630449d3edfd4dcddd2eab9e2fc2f854de5f17f58742fa3b55a090a5212d1decfa0c0b43e7466e1b1bb623a3a8ec4ac53adc87b6b905f8676f9");
        let deserialized_signature = Signature::from_str(&value).unwrap();
        assert_eq!(value, format!("{:}", deserialized_signature));
    }
}
