use ethcontract_generate::{Address, Builder};
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

    generate_contract("IERC20", hashmap! {});
    generate_contract(
        "EasyAuction",
        hashmap! {
            4 => Address::from_str("55a9d7BCd53FF54B146C52Ec52b94C183636DA93").unwrap(),
        },
    );
}

fn generate_contract(name: &str, deployment_overrides: HashMap<u32, Address>) {
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

    for (network_id, address) in deployment_overrides.into_iter() {
        builder = builder.add_deployment(network_id, address);
    }

    builder
        .generate()
        .unwrap()
        .write_to_file(Path::new(&dest).join(format!("{}.rs", name)))
        .unwrap();
}
