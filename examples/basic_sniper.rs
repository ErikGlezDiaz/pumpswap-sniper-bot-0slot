use pumpswap_sniper_bot::*;
use log::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    info!("Starting basic PumpSwap sniper example");
    
    // Load configuration
    let config = Config::load("config.toml")?;
    let config = Arc::new(RwLock::new(config));
    
    // Create sniper bot
    let mut sniper = SniperBot::new(config.clone()).await?;
    
    info!("Sniper bot initialized successfully");
    
    // Start the sniper bot
    sniper.start().await?;
    
    info!("Sniper bot completed");
    Ok(())
}
