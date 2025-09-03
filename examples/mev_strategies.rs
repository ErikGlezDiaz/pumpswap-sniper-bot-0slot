use pumpswap_sniper_bot::*;
use log::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    info!("Starting MEV strategies example");
    
    // Load configuration with MEV enabled
    let mut config = Config::load("config.toml")?;
    config.enable_mev = true;
    config.mev_strategies = vec![
        "arbitrage".to_string(),
        "frontrun".to_string(),
        "backrun".to_string(),
        "sandwich".to_string(),
    ];
    config.max_mev_profit = 500.0;
    
    let config = Arc::new(RwLock::new(config));
    
    // Create MEV detector
    let mut mev_detector = MEVDetector::new(config.clone());
    
    // Simulate some token listings and price updates
    let token_listings = vec![
        TokenListing {
            token_address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            token_symbol: "USDC".to_string(),
            token_name: "USD Coin".to_string(),
            timestamp: 1640995200,
            creator: "creator_address".to_string(),
            initial_liquidity: 1000000000000, // 1000 SOL
            pool_address: "pool_address".to_string(),
            metadata: TokenMetadata {
                address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                decimals: 6,
                logo_uri: "".to_string(),
                description: "".to_string(),
                website: "".to_string(),
                twitter: "".to_string(),
                telegram: "".to_string(),
                verified: true,
                market_cap: 1000000000,
                total_supply: 1000000000,
            },
        },
    ];
    
    let price_updates = vec![
        PriceUpdate {
            token_address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            price_usd: 1.0,
            price_sol: 0.0001,
            liquidity_usd: 1000000.0,
            volume_1h: 50000.0,
            timestamp: 1640995200,
            change_type: PriceChangeType::Increase,
        },
    ];
    
    // Analyze opportunities
    let opportunities = mev_detector.analyze_opportunities(&token_listings, &price_updates).await?;
    
    info!("Found {} MEV opportunities", opportunities.len());
    
    for (i, signal) in opportunities.iter().enumerate() {
        info!("Opportunity {}: {:?} - Expected profit: {} SOL", 
              i + 1, signal.opportunity.strategy, signal.opportunity.expected_profit);
    }
    
    // Create sniper bot
    let mut sniper = SniperBot::new(config.clone()).await?;
    
    info!("Starting MEV sniper bot");
    
    // Start the sniper bot
    sniper.start().await?;
    
    info!("MEV sniper bot completed");
    Ok(())
}
