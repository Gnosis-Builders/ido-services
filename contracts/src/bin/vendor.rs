//! This script is used to vendor Truffle JSON artifacts to be used for code
//! generation with `ethcontract`. This is done instead of fetching contracts
//! at build time to reduce the risk of failure.

use anyhow::Result;
use contracts::paths;
use env_logger::Env;
use ethcontract_generate::Source;
use serde_json::{Map, Value};
use std::fs;

// npm path and local file name
const NPM_CONTRACTS: &[(&str, &str)] = &[
    (
        "@openzeppelin/contracts@3.3.0/build/contracts/ERC20.json",
        "ERC20.json",
    ),
    (
        "@gnosis.pm/ido-contracts@0.5.0/build/artifacts/contracts/test/ERC20Mintable.sol/ERC20Mintable.json",
        "ERC20Mintable.json",
    ),
    (
        "@gnosis.pm/ido-contracts@0.5.0/deployments/rinkeby/EasyAuction.json",
        "EasyAuction.json",
    ),
    (
        "@gnosis.pm/ido-contracts@0.5.0/deployments/rinkeby/AllowListOffChainManaged.json",
        "AllowListOffChainManaged.json",
    ),
    (
        "@gnosis.pm/ido-contracts@0.5.0/deployments/rinkeby/DepositAndPlaceOrder.json",
        "DepositAndPlaceOrder.json",
    ),
    (
        "canonical-weth@1.4.0/build/contracts/WETH9.json",
        "WETH9.json",
    ),
];

fn main() {
    env_logger::init_from_env(Env::default().default_filter_or("warn,vendor=info"));

    if let Err(err) = run() {
        log::error!("Error vendoring contracts: {:?}", err);
        std::process::exit(-1);
    }
}

fn run() -> Result<()> {
    let artifacts = paths::contract_artifacts_dir();
    fs::create_dir_all(&artifacts)?;

    log::info!("vendoring contract artifacts to '{}'", artifacts.display());
    for (npm_path, local_path) in NPM_CONTRACTS {
        log::info!("retrieving {}", npm_path);
        let source = Source::npm(npm_path.to_string());
        let artifact_json = source.artifact_json()?;

        log::debug!("pruning artifact JSON");
        let pruned_artifact_json = {
            let mut json = serde_json::from_str::<Value>(&artifact_json)?;
            let mut pruned = Map::new();
            for property in &[
                "abi",
                "bytecode",
                "contractName",
                "devdoc",
                "networks",
                "userdoc",
            ] {
                if let Some(value) = json.get_mut(property) {
                    pruned.insert(property.to_string(), value.take());
                }
            }
            serde_json::to_string(&pruned)?
        };

        let path = artifacts.join(local_path);
        log::debug!("saving artifact to {}", path.display());
        fs::write(path, pruned_artifact_json)?;
    }

    Ok(())
}
