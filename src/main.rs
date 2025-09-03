use clap::Parser;
use log::{info, error};
use std::sync::Arc;
use tokio::sync::RwLock;

mod config;
mod grpc_client;
mod jito_client;
mod nozomi_client;
mod mev_detector;
mod sniper;
mod monitoring;
mod utils;

use config::Config;
use sniper::SniperBot;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: String,
    
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
    
    /// Target token addresses to monitor
    #[arg(short, long)]
    tokens: Vec<String>,
    
    /// Minimum liquidity threshold (in SOL)
    #[arg(long, default_value = "10.0")]
    min_liquidity: f64,
    
    /// Maximum slippage percentage
    #[arg(long, default_value = "5.0")]
    max_slippage: f64,
    
    /// Use Jito confirmation service
    #[arg(long)]
    use_jito: bool,
    
    /// Use Nozomi confirmation service
    #[arg(long)]
    use_nozomi: bool,
    
    /// Maximum gas price in lamports
    #[arg(long, default_value = "1000000")]
    max_gas_price: u64,
    
    /// Snipe amount in SOL
    #[arg(long, default_value = "1.0")]
    snipe_amount: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize logging
    let log_level = if args.debug { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
        .init();
    
    info!("Starting PumpSwap 0-Slot Sniper Bot");
    
    // Load configuration
    let config = Config::load(&args.config)?;
    info!("Configuration loaded from {}", args.config);
    
    // Override config with command line arguments
    let mut config = config;
    if !args.tokens.is_empty() {
        config.target_tokens = args.tokens;
    }
    config.min_liquidity = args.min_liquidity;
    config.max_slippage = args.max_slippage;
    config.max_gas_price = args.max_gas_price;
    config.snipe_amount = args.snipe_amount;
    
    // Set confirmation service
    if args.use_jito {
        config.confirmation_service = "jito".to_string();
    } else if args.use_nozomi {
        config.confirmation_service = "nozomi".to_string();
    }
    
    // Create shared configuration
    let config = Arc::new(RwLock::new(config));
    
    // Initialize monitoring
    let monitoring = monitoring::Monitoring::new(config.clone()).await?;
    
    // Start monitoring in background
    let monitoring_handle = tokio::spawn(async move {
        if let Err(e) = monitoring.start().await {
            error!("Monitoring error: {}", e);
        }
    });
    
    // Create and start sniper bot
    let mut sniper = SniperBot::new(config.clone()).await?;
    
    info!("Starting sniper bot with configuration:");
    info!("  Target tokens: {:?}", config.read().await.target_tokens);
    info!("  Min liquidity: {} SOL", config.read().await.min_liquidity);
    info!("  Max slippage: {}%", config.read().await.max_slippage);
    info!("  Confirmation service: {}", config.read().await.confirmation_service);
    info!("  Snipe amount: {} SOL", config.read().await.snipe_amount);
    
    // Start the sniper bot
    match sniper.start().await {
        Ok(_) => {
            info!("Sniper bot completed successfully");
        }
        Err(e) => {
            error!("Sniper bot failed: {}", e);
            return Err(e.into());
        }
    }
    
    // Wait for monitoring to complete
    let _ = monitoring_handle.await;
    
    Ok(())
}
