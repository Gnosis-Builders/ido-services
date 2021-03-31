use ethcontract_generate::{Address, Builder, TransactionHash};
use lazy_static::lazy_static;
use maplit::hashmap;
use primitive_types::H256;
use std::{collections::HashMap, env, fs, path::Path, str::FromStr};

#[path = "src/paths.rs"]
mod paths;

// Todo: duplication from orderbook/main file.
lazy_static! {
    pub static ref EASY_AUCTION_DEPLOYMENT_INFO: HashMap::<u32, (Address, Option<H256>)> = hashmap! {
    1 => (Address::from_str("0b7fFc1f4AD541A4Ed16b40D8c37f0929158D101").unwrap(), Some("0xa7ad659a9762720bd86a30b49a3e139928cc2a27d0863ab78110e19d2bef8a51".parse().unwrap())),
    4 => (Address::from_str("C5992c0e0A3267C7F75493D0F717201E26BE35f7").unwrap(), Some("0xbdd1dde815a908d407ec89fa9bc317d9e33621ccc6452ac0eb00fe2ed0d81ff4".parse().unwrap())),
    100 => (Address::from_str("0b7fFc1f4AD541A4Ed16b40D8c37f0929158D101").unwrap(), Some("0x5af5443ba9add113a42b0219ac8f398c383dc5a3684a221fd24c5655b8316931".parse().unwrap())),
    };
}

fn main() {
    // NOTE: This is a workaround for `rerun-if-changed` directives for
    // non-existant files cause the crate's build unit to get flagged for a
    // rebuild if any files in the workspace change.
    //
    // See:
    // - https://github.com/rust-lang/cargo/issues/6003
    // - https://doc.rust-lang.org/cargo/reference/build-scripts.html#cargorerun-if-changedpath
    println!("cargo:rerun-if-changed=build.rs");

    generate_contract("ERC20", hashmap! {});
    generate_contract("ERC20Mintable", hashmap! {});
    generate_contract("EasyAuction", EASY_AUCTION_DEPLOYMENT_INFO.clone());
    generate_contract(
        "AllowListOffChainManaged",
        hashmap! {
            1 => (Address::from_str("0F4648d997e486cE06577d6Ee2FecBcA84b834F4").unwrap(), Some("0xf1c7cf15be13691a358090065fa8a25005038a8c58eb4a2f882f7fa8dd5b9426".parse().unwrap())),
            4 => (Address::from_str("7C882F296335734B958b35DA6b2595FA00043AE9").unwrap(), Some("0xf1c7cf15be13691a358090065fa8a25005038a8c58eb4a2f882f7fa8dd5b9426".parse().unwrap())),
            100 => (Address::from_str("0F4648d997e486cE06577d6Ee2FecBcA84b834F4").unwrap(), Some("0x9be0d0f472e3a41c1fb314624965fefbdeb0c5ebe3671c59191a794b68265f10".parse().unwrap())),
        },
    );
    generate_contract(
        "DepositAndPlaceOrder",
        hashmap! {
            1 => (Address::from_str("10D15DEA67f7C95e2F9Fe4eCC245a8862b9B5B96").unwrap(), Some("0x8a88034c1d1729c3a72e2d9d0f05056d5e4155f6f1368882e6f743f0fe3d6966".parse().unwrap())),
            4 => (Address::from_str("845AbED0734e39614FEC4245F3F3C88E2da98157").unwrap(), Some("0xdc6b81239087cc685f4bd9f3a9733d3b0fdc54326868dee8b57b4073ef1fc92e".parse().unwrap())),
            100 => (Address::from_str("69BE2732891A10D6d3d00073A834194Ff3EeB71d").unwrap(), Some("0x7d239d8ab763f396739d7988c2bd909851067bf94007b6321d2ef69602104ce6".parse().unwrap())),
        },
    );
    generate_contract(
        "WETH9",
        hashmap! {
            // Rinkeby & Mainnet Addresses are part of the artefact
            100 => (Address::from_str("e91D153E0b41518A2Ce8Dd3D7944Fa863463a97d").unwrap(), Some("0x0c2632fc6588506d3a6a1cdb10140bb9281f898f6c1b532728409c623ca8432b".parse().unwrap())),
        },
    );
}

fn generate_contract(
    name: &str,
    deployment_overrides: HashMap<u32, (Address, Option<TransactionHash>)>,
) {
    let artifact = paths::contract_artifacts_dir().join(format!("{}.json", name));
    let address_file = paths::contract_address_file(name);
    let dest = env::var("OUT_DIR").unwrap();

    println!("cargo:rerun-if-changed={}", artifact.display());
    let mut builder = Builder::new(artifact)
        .with_contract_name_override(Some(name))
        .with_visibility_modifier(Some("pub"))
        .add_event_derive("serde::Deserialize")
        .add_event_derive("serde::Serialize");

    if let Ok(address) = fs::read_to_string(&address_file) {
        println!("cargo:rerun-if-changed={}", address_file.display());
        builder = builder.add_deployment_str(5777, address.trim());
    }

    for (network_id, (address, transaction_hash)) in deployment_overrides.into_iter() {
        builder = builder.add_deployment(network_id, address, transaction_hash);
    }

    builder
        .generate()
        .unwrap()
        .write_to_file(Path::new(&dest).join(format!("{}.rs", name)))
        .unwrap();
}
