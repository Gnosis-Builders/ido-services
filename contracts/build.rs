use ethcontract::common::DeploymentInformation;
use ethcontract_generate::{Address, Builder};
use std::{env, fs, path::Path};

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

    generate_contract("ERC20");
    generate_contract("ERC20Mintable");
    generate_contract_with_config("EasyAuction", |builder| {
        builder
            .with_contract_mod_override(Some("easy_auction"))
            .add_deployment(
                1,
                addr("0b7fFc1f4AD541A4Ed16b40D8c37f0929158D101"),
                Some(tx(
                    "0xa7ad659a9762720bd86a30b49a3e139928cc2a27d0863ab78110e19d2bef8a51",
                )),
            )
            .add_deployment(
                4,
                addr("C5992c0e0A3267C7F75493D0F717201E26BE35f7"),
                Some(tx(
                    "0xbdd1dde815a908d407ec89fa9bc317d9e33621ccc6452ac0eb00fe2ed0d81ff4",
                )),
            )
            .add_deployment(
                100,
                addr("0b7fFc1f4AD541A4Ed16b40D8c37f0929158D101"),
                Some(tx(
                    "0x5af5443ba9add113a42b0219ac8f398c383dc5a3684a221fd24c5655b8316931",
                )),
            )
            .add_deployment(
                137,
                addr("0b7fFc1f4AD541A4Ed16b40D8c37f0929158D101"),
                Some(tx(
                    "0x6093f70c46350202181e9b0edfcf8f0e966ddddeb8b24e8b73dd2ab636c1ce87",
                )),
            )
    });
    generate_contract_with_config("AllowListOffChainManaged", |builder| {
        builder
            .with_contract_mod_override(Some("allow_list_off_chain_managed"))
            .add_deployment(
                1,
                addr("0F4648d997e486cE06577d6Ee2FecBcA84b834F4"),
                Some(tx(
                    "0xf1c7cf15be13691a358090065fa8a25005038a8c58eb4a2f882f7fa8dd5b9426",
                )),
            )
            .add_deployment(
                4,
                addr("7C882F296335734B958b35DA6b2595FA00043AE9"),
                Some(tx(
                    "0xf1c7cf15be13691a358090065fa8a25005038a8c58eb4a2f882f7fa8dd5b9426",
                )),
            )
            .add_deployment(
                100,
                addr("0F4648d997e486cE06577d6Ee2FecBcA84b834F4"),
                Some(tx(
                    "0x9be0d0f472e3a41c1fb314624965fefbdeb0c5ebe3671c59191a794b68265f10",
                )),
            )
    });
    generate_contract_with_config("DepositAndPlaceOrder", |builder| {
        builder
            .with_contract_mod_override(Some("deposit_and_place_order"))
            .add_deployment(
                1,
                addr("10D15DEA67f7C95e2F9Fe4eCC245a8862b9B5B96"),
                Some(tx(
                    "0x8a88034c1d1729c3a72e2d9d0f05056d5e4155f6f1368882e6f743f0fe3d6966",
                )),
            )
            .add_deployment(
                4,
                addr("845AbED0734e39614FEC4245F3F3C88E2da98157"),
                Some(tx(
                    "0xdc6b81239087cc685f4bd9f3a9733d3b0fdc54326868dee8b57b4073ef1fc92e",
                )),
            )
            .add_deployment(
                100,
                addr("69BE2732891A10D6d3d00073A834194Ff3EeB71d"),
                Some(tx(
                    "0x7d239d8ab763f396739d7988c2bd909851067bf94007b6321d2ef69602104ce6",
                )),
            )
    });
    generate_contract_with_config("WETH9", |builder| {
        builder
            // Rinkeby & Mainnet Addresses are part of the artefact
            .with_contract_mod_override(Some("weth9"))
            .add_deployment(
                100,
                addr("e91D153E0b41518A2Ce8Dd3D7944Fa863463a97d"),
                Some(tx(
                    "0x0c2632fc6588506d3a6a1cdb10140bb9281f898f6c1b532728409c623ca8432b",
                )),
            )
    });
}

fn generate_contract(name: &str) {
    generate_contract_with_config(name, |builder| builder)
}

fn generate_contract_with_config(name: &str, config: impl FnOnce(Builder) -> Builder) {
    let artifact = paths::contract_artifacts_dir()
        .join(name)
        .with_extension("json");
    let address_file = paths::contract_address_file(name);
    let dest = env::var("OUT_DIR").unwrap();

    println!("cargo:rerun-if-changed={}", artifact.display());
    let mut builder = Builder::new(artifact)
        .with_contract_name_override(Some(name))
        .with_visibility_modifier(Some("pub"));

    if let Ok(address) = fs::read_to_string(&address_file) {
        println!("cargo:rerun-if-changed={}", address_file.display());
        builder = builder.add_deployment_str(5777, address.trim());
    }

    config(builder)
        .generate()
        .unwrap()
        .write_to_file(Path::new(&dest).join(format!("{}.rs", name)))
        .unwrap();
}

fn addr(s: &str) -> Address {
    s.parse().unwrap()
}

fn tx(s: &str) -> DeploymentInformation {
    DeploymentInformation::TransactionHash(s.parse().unwrap())
}
