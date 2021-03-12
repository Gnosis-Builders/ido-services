use ethcontract_generate::{Address, Builder, TransactionHash};
use maplit::hashmap;
use std::{collections::HashMap, env, fs, path::Path, str::FromStr};

#[path = "src/paths.rs"]
mod paths;

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
    generate_contract(
        "EasyAuction",
        hashmap! {
            4 => (Address::from_str("307C1384EFeF241d6CBBFb1F85a04C54307Ac9F6").unwrap(), Some("0xecf8358d08dfdbd9549c0affa2226b062fe78867f156a258bd9da1e05ad842aa".parse().unwrap())),
            100 => (Address::from_str("9BacE46438b3f3e0c06d67f5C1743826EE8e87DA").unwrap(), Some("0x7304d6dfe40a8b5a97c6579743733139dd50c3b4a7d39181fd7c24ac28c3986f".parse().unwrap())),
        },
    );
    generate_contract(
        "AllowListOffChainManaged",
        hashmap! {
            4 => (Address::from_str("80b8AcA4689EC911F048c4E0976892cCDE14031E").unwrap(), Some("0x42930a3432aea531ff46181b9244dded7e79d7e8edf890802a60f174aef70abd".parse().unwrap())),
            100 => (Address::from_str("80b8AcA4689EC911F048c4E0976892cCDE14031E").unwrap(), Some("0xecf8358d08dfdbd9549c0affa2226b062fe78867f156a258bd9da1e05ad842aa".parse().unwrap())),
        },
    );
    generate_contract(
        "DepositAndPlaceOrder",
        hashmap! {
            4 => (Address::from_str("6a357cb2ed9230eFAD971d83dcc54981636aEA97").unwrap(), Some("0xdc6b81239087cc685f4bd9f3a9733d3b0fdc54326868dee8b57b4073ef1fc92e".parse().unwrap())),
            100 => (Address::from_str("69BE2732891A10D6d3d00073A834194Ff3EeB71d").unwrap(), Some("0x2ca9369a435d91e272881aa5c6393df176aa2a5e48bfb19c45fabe8b1d3dea49".parse().unwrap())),
        },
    );
    generate_contract(
        "WETH9",
        hashmap! {
            // Rinkeby & Mainnet Addresses are part of the artefact
            100 => (Address::from_str("e91D153E0b41518A2Ce8Dd3D7944Fa863463a97d").unwrap(), None),
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
