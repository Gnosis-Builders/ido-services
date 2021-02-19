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
            4 => (Address::from_str("99e63218201e44549AB8a6Fa220e1018FDB48f79").unwrap(), Some("0x1719a22ec302cc15f2130731c88580dbd19be8292573b0b7a2d1455c41ab6867".parse().unwrap())),
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
