use pumpswap_sniper_bot::*;
use log::info;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    info!("Starting monitoring example");
    
    // Load configuration
    let config = Config::load("config.toml")?;
    let config = Arc::new(RwLock::new(config));
    
    // Create monitoring system
    let mut monitoring = Monitoring::new(config.clone()).await?;
    
    // Create trade logger
    let trade_logger = TradeLogger::new(config.clone());
    
    // Simulate some trades
    trade_logger.log_trade_start("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", 1000000, "arbitrage");
    trade_logger.log_trade_success("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", 0.5, 50000, 150.0);
    
    trade_logger.log_mev_opportunity("frontrun", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", 1.2);
    trade_logger.log_mev_execution("frontrun", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", 1.1);
    
    trade_logger.log_bundle_submission("bundle_123", 3);
    trade_logger.log_bundle_confirmation("bundle_123", 250.0);
    
    trade_logger.log_price_impact("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", 0.5);
    trade_logger.log_slippage("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", 0.2);
    
    // Start monitoring
    let monitoring_handle = tokio::spawn(async move {
        if let Err(e) = monitoring.start().await {
            eprintln!("Monitoring error: {}", e);
        }
    });
    
    // Let monitoring run for a bit
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    // Stop monitoring
    monitoring_handle.abort();
    
    info!("Monitoring example completed");
    Ok(())
}
