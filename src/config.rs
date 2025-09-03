use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // PumpSwap gRPC configuration
    pub pumpswap_grpc_url: String,
    pub pumpswap_api_key: Option<String>,
    
    // Solana RPC configuration
    pub solana_rpc_url: String,
    pub solana_ws_url: Option<String>,
    
    // Wallet configuration
    pub private_key: String,
    pub wallet_address: Option<String>,
    
    // Sniper configuration
    pub target_tokens: Vec<String>,
    pub min_liquidity: f64,
    pub max_slippage: f64,
    pub snipe_amount: f64,
    pub max_gas_price: u64,
    
    // MEV configuration
    pub enable_mev: bool,
    pub mev_strategies: Vec<String>,
    pub max_mev_profit: f64,
    
    // Confirmation service
    pub confirmation_service: String, // "jito" or "nozomi"
    
    // Jito configuration
    pub jito_url: String,
    pub jito_tip_account: String,
    pub jito_tip_amount: u64,
    
    // Nozomi configuration
    pub nozomi_url: String,
    pub nozomi_api_key: Option<String>,
    
    // Performance settings
    pub max_concurrent_trades: usize,
    pub transaction_timeout: u64,
    pub retry_attempts: u32,
    pub retry_delay: u64,
    
    // Monitoring
    pub enable_metrics: bool,
    pub metrics_port: u16,
    pub log_level: String,
    
    // Risk management
    pub max_daily_loss: f64,
    pub max_position_size: f64,
    pub stop_loss_percentage: f64,
    pub take_profit_percentage: f64,
    
    // Advanced settings
    pub priority_fee_multiplier: f64,
    pub bundle_timeout: u64,
    pub max_bundle_size: usize,
    pub enable_frontrunning_protection: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pumpswap_grpc_url: "https://grpc.pumpswap.fun:443".to_string(),
            pumpswap_api_key: None,
            
            solana_rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            solana_ws_url: Some("wss://api.mainnet-beta.solana.com".to_string()),
            
            private_key: String::new(),
            wallet_address: None,
            
            target_tokens: vec![],
            min_liquidity: 10.0,
            max_slippage: 5.0,
            snipe_amount: 1.0,
            max_gas_price: 1000000,
            
            enable_mev: true,
            mev_strategies: vec!["arbitrage".to_string(), "frontrun".to_string()],
            max_mev_profit: 1000.0,
            
            confirmation_service: "jito".to_string(),
            
            jito_url: "https://mainnet.block-engine.jito.wtf".to_string(),
            jito_tip_account: "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY".to_string(),
            jito_tip_amount: 10000,
            
            nozomi_url: "https://api.nozomi.com".to_string(),
            nozomi_api_key: None,
            
            max_concurrent_trades: 5,
            transaction_timeout: 30,
            retry_attempts: 3,
            retry_delay: 1000,
            
            enable_metrics: true,
            metrics_port: 9090,
            log_level: "info".to_string(),
            
            max_daily_loss: 100.0,
            max_position_size: 10.0,
            stop_loss_percentage: 10.0,
            take_profit_percentage: 50.0,
            
            priority_fee_multiplier: 1.5,
            bundle_timeout: 5000,
            max_bundle_size: 10,
            enable_frontrunning_protection: true,
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        if std::path::Path::new(path).exists() {
            let content = std::fs::read_to_string(path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            // Create default config file
            let default_config = Config::default();
            let content = toml::to_string_pretty(&default_config)?;
            std::fs::write(path, content)?;
            log::info!("Created default configuration file: {}", path);
            Ok(default_config)
        }
    }
    
    pub fn save(&self, path: &str) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    pub fn validate(&self) -> Result<()> {
        if self.private_key.is_empty() {
            return Err(anyhow::anyhow!("Private key is required"));
        }
        
        if self.target_tokens.is_empty() {
            return Err(anyhow::anyhow!("At least one target token is required"));
        }
        
        if self.min_liquidity <= 0.0 {
            return Err(anyhow::anyhow!("Minimum liquidity must be positive"));
        }
        
        if self.max_slippage <= 0.0 || self.max_slippage > 100.0 {
            return Err(anyhow::anyhow!("Maximum slippage must be between 0 and 100"));
        }
        
        if self.snipe_amount <= 0.0 {
            return Err(anyhow::anyhow!("Snipe amount must be positive"));
        }
        
        if !["jito", "nozomi"].contains(&self.confirmation_service.as_str()) {
            return Err(anyhow::anyhow!("Confirmation service must be 'jito' or 'nozomi'"));
        }
        
        Ok(())
    }
    
    pub fn get_mev_strategies(&self) -> Vec<MEVStrategy> {
        self.mev_strategies
            .iter()
            .filter_map(|s| match s.as_str() {
                "arbitrage" => Some(MEVStrategy::Arbitrage),
                "frontrun" => Some(MEVStrategy::FrontRun),
                "backrun" => Some(MEVStrategy::BackRun),
                "sandwich" => Some(MEVStrategy::Sandwich),
                "liquidation" => Some(MEVStrategy::Liquidation),
                _ => None,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MEVStrategy {
    Arbitrage,
    FrontRun,
    BackRun,
    Sandwich,
    Liquidation,
}

impl MEVStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            MEVStrategy::Arbitrage => "arbitrage",
            MEVStrategy::FrontRun => "frontrun",
            MEVStrategy::BackRun => "backrun",
            MEVStrategy::Sandwich => "sandwich",
            MEVStrategy::Liquidation => "liquidation",
        }
    }
}
