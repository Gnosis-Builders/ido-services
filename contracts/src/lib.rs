#[cfg(feature = "bin")]
pub mod paths;

include!(concat!(env!("OUT_DIR"), "/ERC20.rs"));
include!(concat!(env!("OUT_DIR"), "/ERC20Mintable.rs"));
include!(concat!(env!("OUT_DIR"), "/EasyAuction.rs"));
