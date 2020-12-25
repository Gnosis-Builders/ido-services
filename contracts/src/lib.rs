#[cfg(feature = "bin")]
pub mod paths;

include!(concat!(env!("OUT_DIR"), "/IERC20.rs"));
include!(concat!(env!("OUT_DIR"), "/EasyAuction.rs"));
