pub mod config;
pub mod grpc_client;
pub mod jito_client;
pub mod nozomi_client;
pub mod mev_detector;
pub mod sniper;
pub mod monitoring;
pub mod utils;

// Re-export main types for easier access
pub use config::{Config, MEVStrategy};
pub use grpc_client::{PumpSwapGrpcClient, TokenListingStream, PriceUpdateStream};
pub use jito_client::{JitoClient, BundleManager, Bundle, BundleTransaction};
pub use nozomi_client::{NozomiClient, NozomiManager, NozomiSubmission};
pub use mev_detector::{MEVDetector, MEVOpportunity, MEVSignal, MEVPriority};
pub use sniper::SniperBot;
pub use monitoring::{Monitoring, TradeLogger};
pub use utils::*;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Initialize the sniper bot library
pub fn init() {
    env_logger::init();
}

/// Get library information
pub fn get_info() -> std::collections::HashMap<String, String> {
    let mut info = std::collections::HashMap::new();
    info.insert("name".to_string(), NAME.to_string());
    info.insert("version".to_string(), VERSION.to_string());
    info.insert("rust_version".to_string(), env!("RUST_VERSION").to_string());
    info
}
