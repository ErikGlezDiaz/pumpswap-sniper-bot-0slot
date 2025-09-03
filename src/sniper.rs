use anyhow::Result;
use log::{debug, error, info, warn};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, Semaphore};

use crate::config::Config;
use crate::grpc_client::{PumpSwapGrpcClient, TokenListingStream, PriceUpdateStream};
use crate::jito_client::BundleManager;
use crate::mev_detector::{MEVDetector, MEVSignal, MEVPriority};
use crate::nozomi_client::NozomiManager;
use crate::proto::pumpswap::*;

pub struct SniperBot {
    config: Arc<RwLock<Config>>,
    grpc_client: PumpSwapGrpcClient,
    jito_manager: Option<BundleManager>,
    nozomi_manager: Option<NozomiManager>,
    mev_detector: MEVDetector,
    wallet: Keypair,
    active_trades: std::collections::HashMap<String, ActiveTrade>,
    trade_semaphore: Arc<Semaphore>,
}

#[derive(Debug, Clone)]
struct ActiveTrade {
    pub token_address: String,
    pub amount: u64,
    pub target_price: f64,
    pub max_slippage: f64,
    pub created_at: u64,
    pub status: TradeStatus,
}

#[derive(Debug, Clone, PartialEq)]
enum TradeStatus {
    Pending,
    Executing,
    Completed,
    Failed,
    Cancelled,
}

impl SniperBot {
    pub async fn new(config: Arc<RwLock<Config>>) -> Result<Self> {
        // Initialize gRPC client
        let grpc_client = PumpSwapGrpcClient::new(config.clone()).await?;
        
        // Initialize confirmation service managers
        let config_guard = config.read().await;
        let jito_manager = if config_guard.confirmation_service == "jito" {
            Some(BundleManager::new(config.clone())?)
        } else {
            None
        };
        
        let nozomi_manager = if config_guard.confirmation_service == "nozomi" {
            Some(NozomiManager::new(config.clone())?)
        } else {
            None
        };
        
        // Initialize wallet
        let private_key = &config_guard.private_key;
        let wallet = if private_key.starts_with('[') {
            // Array format
            let bytes: Vec<u8> = serde_json::from_str(private_key)?;
            Keypair::from_bytes(&bytes)?
        } else {
            // Base58 format
            let bytes = bs58::decode(private_key).into_vec()?;
            Keypair::from_bytes(&bytes)?
        };
        
        let max_concurrent_trades = config_guard.max_concurrent_trades;
        drop(config_guard);
        
        // Initialize MEV detector
        let mev_detector = MEVDetector::new(config.clone());
        
        Ok(Self {
            config,
            grpc_client,
            jito_manager,
            nozomi_manager,
            mev_detector,
            wallet,
            active_trades: std::collections::HashMap::new(),
            trade_semaphore: Arc::new(Semaphore::new(max_concurrent_trades)),
        })
    }
    
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting PumpSwap 0-Slot Sniper Bot");
        
        // Start token listing stream
        let token_stream_handle = {
            let config = self.config.clone();
            let mev_detector = self.mev_detector.clone();
            let wallet = self.wallet.clone();
            let trade_semaphore = self.trade_semaphore.clone();
            let mut jito_manager = self.jito_manager.clone();
            let mut nozomi_manager = self.nozomi_manager.clone();
            
            tokio::spawn(async move {
                let mut stream = TokenListingStream::new(config.clone());
                if let Err(e) = stream.start_streaming(|listing| {
                    let config = config.clone();
                    let mev_detector = mev_detector.clone();
                    let wallet = wallet.clone();
                    let trade_semaphore = trade_semaphore.clone();
                    let mut jito_manager = jito_manager.clone();
                    let mut nozomi_manager = nozomi_manager.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = Self::process_new_listing(
                            listing,
                            config,
                            mev_detector,
                            wallet,
                            trade_semaphore,
                            jito_manager,
                            nozomi_manager,
                        ).await {
                            error!("Error processing new listing: {}", e);
                        }
                    });
                    
                    Ok(true) // Continue streaming
                }).await {
                    error!("Token listing stream error: {}", e);
                }
            })
        };
        
        // Start price update stream
        let price_stream_handle = {
            let config = self.config.clone();
            let target_tokens = self.config.read().await.target_tokens.clone();
            let mev_detector = self.mev_detector.clone();
            let wallet = self.wallet.clone();
            let trade_semaphore = self.trade_semaphore.clone();
            let mut jito_manager = self.jito_manager.clone();
            let mut nozomi_manager = self.nozomi_manager.clone();
            
            tokio::spawn(async move {
                let mut stream = PriceUpdateStream::new(config.clone(), target_tokens).await.unwrap();
                if let Err(e) = stream.start_streaming(|price_update| {
                    let config = config.clone();
                    let mev_detector = mev_detector.clone();
                    let wallet = wallet.clone();
                    let trade_semaphore = trade_semaphore.clone();
                    let mut jito_manager = jito_manager.clone();
                    let mut nozomi_manager = nozomi_manager.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = Self::process_price_update(
                            price_update,
                            config,
                            mev_detector,
                            wallet,
                            trade_semaphore,
                            jito_manager,
                            nozomi_manager,
                        ).await {
                            error!("Error processing price update: {}", e);
                        }
                    });
                    
                    Ok(true) // Continue streaming
                }).await {
                    error!("Price update stream error: {}", e);
                }
            })
        };
        
        // Start trade monitoring
        let trade_monitor_handle = {
            let active_trades = Arc::new(RwLock::new(self.active_trades.clone()));
            let config = self.config.clone();
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(1));
                loop {
                    interval.tick().await;
                    
                    // Clean up completed trades
                    let mut trades = active_trades.write().await;
                    trades.retain(|_, trade| {
                        let elapsed = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs() - trade.created_at;
                        
                        elapsed < 300 // Keep trades for 5 minutes
                    });
                }
            })
        };
        
        info!("Sniper bot started successfully");
        
        // Wait for streams to complete
        tokio::select! {
            _ = token_stream_handle => {
                info!("Token listing stream completed");
            }
            _ = price_stream_handle => {
                info!("Price update stream completed");
            }
            _ = trade_monitor_handle => {
                info!("Trade monitor completed");
            }
        }
        
        Ok(())
    }
    
    async fn process_new_listing(
        listing: TokenListing,
        config: Arc<RwLock<Config>>,
        mut mev_detector: MEVDetector,
        wallet: Keypair,
        trade_semaphore: Arc<Semaphore>,
        mut jito_manager: Option<BundleManager>,
        mut nozomi_manager: Option<NozomiManager>,
    ) -> Result<()> {
        info!("Processing new listing: {} ({})", listing.token_symbol, listing.token_address);
        
        // Check if we should snipe this token
        let should_snipe = Self::should_snipe_token(&listing, &config).await?;
        
        if should_snipe {
            info!("Token {} meets snipe criteria, executing snipe", listing.token_address);
            
            // Acquire semaphore to limit concurrent trades
            let _permit = trade_semaphore.acquire().await?;
            
            // Execute snipe
            if let Err(e) = Self::execute_snipe(
                &listing,
                &config,
                &wallet,
                &mut jito_manager,
                &mut nozomi_manager,
            ).await {
                error!("Snipe execution failed: {}", e);
            }
        }
        
        // Analyze for MEV opportunities
        if config.read().await.enable_mev {
            let opportunities = mev_detector.analyze_opportunities(&[listing], &[]).await?;
            
            for signal in opportunities {
                if signal.priority >= MEVPriority::High {
                    info!("High priority MEV opportunity detected: {:?}", signal.opportunity.strategy);
                    
                    // Execute MEV strategy
                    if let Err(e) = Self::execute_mev_strategy(
                        &signal,
                        &config,
                        &wallet,
                        &mut jito_manager,
                        &mut nozomi_manager,
                    ).await {
                        error!("MEV strategy execution failed: {}", e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn process_price_update(
        price_update: PriceUpdate,
        config: Arc<RwLock<Config>>,
        mut mev_detector: MEVDetector,
        wallet: Keypair,
        trade_semaphore: Arc<Semaphore>,
        mut jito_manager: Option<BundleManager>,
        mut nozomi_manager: Option<NozomiManager>,
    ) -> Result<()> {
        debug!("Processing price update for {}: ${:.6}", price_update.token_address, price_update.price_usd);
        
        // Analyze for MEV opportunities
        if config.read().await.enable_mev {
            let opportunities = mev_detector.analyze_opportunities(&[], &[price_update]).await?;
            
            for signal in opportunities {
                if signal.priority >= MEVPriority::Medium {
                    info!("MEV opportunity detected: {:?}", signal.opportunity.strategy);
                    
                    // Acquire semaphore to limit concurrent trades
                    let _permit = trade_semaphore.acquire().await?;
                    
                    // Execute MEV strategy
                    if let Err(e) = Self::execute_mev_strategy(
                        &signal,
                        &config,
                        &wallet,
                        &mut jito_manager,
                        &mut nozomi_manager,
                    ).await {
                        error!("MEV strategy execution failed: {}", e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn should_snipe_token(listing: &TokenListing, config: &Arc<RwLock<Config>>) -> Result<bool> {
        let config_guard = config.read().await;
        
        // Check minimum liquidity
        if listing.initial_liquidity < (config_guard.min_liquidity * 1e9) as u64 {
            return Ok(false);
        }
        
        // Check if token is in target list
        if !config_guard.target_tokens.is_empty() && !config_guard.target_tokens.contains(&listing.token_address) {
            return Ok(false);
        }
        
        // Additional criteria can be added here
        // - Token metadata validation
        // - Creator reputation
        // - Liquidity distribution
        // - etc.
        
        Ok(true)
    }
    
    async fn execute_snipe(
        listing: &TokenListing,
        config: &Arc<RwLock<Config>>,
        wallet: &Keypair,
        jito_manager: &mut Option<BundleManager>,
        nozomi_manager: &mut Option<NozomiManager>,
    ) -> Result<()> {
        let config_guard = config.read().await;
        let snipe_amount = (config_guard.snipe_amount * 1e9) as u64; // Convert SOL to lamports
        
        info!("Executing snipe for {} with {} SOL", listing.token_address, config_guard.snipe_amount);
        
        // Create buy transaction
        let transaction = Self::create_buy_transaction(
            &listing.token_address,
            &listing.pool_address,
            snipe_amount,
            &config_guard.max_slippage,
            wallet,
        ).await?;
        
        // Submit transaction based on confirmation service
        match config_guard.confirmation_service.as_str() {
            "jito" => {
                if let Some(jito_manager) = jito_manager {
                    let submission_id = jito_manager.submit_transaction(&transaction).await?;
                    info!("Snipe transaction submitted to Jito: {}", submission_id);
                } else {
                    return Err(anyhow::anyhow!("Jito manager not initialized"));
                }
            }
            "nozomi" => {
                if let Some(nozomi_manager) = nozomi_manager {
                    let submission_id = nozomi_manager.submit_transaction(&transaction).await?;
                    info!("Snipe transaction submitted to Nozomi: {}", submission_id);
                } else {
                    return Err(anyhow::anyhow!("Nozomi manager not initialized"));
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown confirmation service: {}", config_guard.confirmation_service));
            }
        }
        
        Ok(())
    }
    
    async fn execute_mev_strategy(
        signal: &MEVSignal,
        config: &Arc<RwLock<Config>>,
        wallet: &Keypair,
        jito_manager: &mut Option<BundleManager>,
        nozomi_manager: &mut Option<NozomiManager>,
    ) -> Result<()> {
        info!("Executing MEV strategy: {:?} for token {}", signal.opportunity.strategy, signal.opportunity.token_address);
        
        // Create transactions for the MEV strategy
        let transactions = Self::create_mev_transactions(signal, wallet).await?;
        
        // Submit transactions based on confirmation service
        let config_guard = config.read().await;
        match config_guard.confirmation_service.as_str() {
            "jito" => {
                if let Some(jito_manager) = jito_manager {
                    let submission_id = jito_manager.submit_transaction_batch(transactions).await?;
                    info!("MEV transactions submitted to Jito: {}", submission_id);
                } else {
                    return Err(anyhow::anyhow!("Jito manager not initialized"));
                }
            }
            "nozomi" => {
                if let Some(nozomi_manager) = nozomi_manager {
                    let submission_id = nozomi_manager.submit_transaction_batch(transactions).await?;
                    info!("MEV transactions submitted to Nozomi: {}", submission_id);
                } else {
                    return Err(anyhow::anyhow!("Nozomi manager not initialized"));
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown confirmation service: {}", config_guard.confirmation_service));
            }
        }
        
        Ok(())
    }
    
    async fn create_buy_transaction(
        token_address: &str,
        pool_address: &str,
        amount: u64,
        max_slippage: &f64,
        wallet: &Keypair,
    ) -> Result<Transaction> {
        // This would create the actual buy transaction
        // For now, we'll create a placeholder transaction
        
        let token_pubkey: Pubkey = token_address.parse()?;
        let pool_pubkey: Pubkey = pool_address.parse()?;
        
        // Create swap instruction (placeholder)
        let instruction = Instruction {
            program_id: pool_pubkey, // This would be the actual swap program
            accounts: vec![], // This would contain the actual accounts
            data: vec![], // This would contain the actual instruction data
        };
        
        // Create transaction
        let message = solana_sdk::message::Message::new(&[instruction], Some(&wallet.pubkey()));
        let transaction = Transaction::new(&[wallet], message, solana_sdk::hash::Hash::default());
        
        Ok(transaction)
    }
    
    async fn create_mev_transactions(
        signal: &MEVSignal,
        wallet: &Keypair,
    ) -> Result<Vec<Transaction>> {
        // This would create the actual MEV transactions based on the strategy
        // For now, we'll create placeholder transactions
        
        let mut transactions = Vec::new();
        
        match signal.opportunity.strategy {
            crate::config::MEVStrategy::Arbitrage => {
                // Create arbitrage transactions
                transactions.push(Self::create_buy_transaction(
                    &signal.opportunity.token_address,
                    &signal.opportunity.pool_address,
                    1000000, // 0.001 SOL
                    &signal.execution_plan.max_slippage,
                    wallet,
                ).await?);
            }
            crate::config::MEVStrategy::FrontRun => {
                // Create front-running transactions
                transactions.push(Self::create_buy_transaction(
                    &signal.opportunity.token_address,
                    &signal.opportunity.pool_address,
                    2000000, // 0.002 SOL
                    &signal.execution_plan.max_slippage,
                    wallet,
                ).await?);
            }
            crate::config::MEVStrategy::Sandwich => {
                // Create sandwich attack transactions
                transactions.push(Self::create_buy_transaction(
                    &signal.opportunity.token_address,
                    &signal.opportunity.pool_address,
                    1500000, // 0.0015 SOL
                    &signal.execution_plan.max_slippage,
                    wallet,
                ).await?);
            }
            crate::config::MEVStrategy::BackRun => {
                // Create back-running transactions
                transactions.push(Self::create_buy_transaction(
                    &signal.opportunity.token_address,
                    &signal.opportunity.pool_address,
                    800000, // 0.0008 SOL
                    &signal.execution_plan.max_slippage,
                    wallet,
                ).await?);
            }
            crate::config::MEVStrategy::Liquidation => {
                // Create liquidation transactions
                transactions.push(Self::create_buy_transaction(
                    &signal.opportunity.token_address,
                    &signal.opportunity.pool_address,
                    5000000, // 0.005 SOL
                    &signal.execution_plan.max_slippage,
                    wallet,
                ).await?);
            }
        }
        
        Ok(transactions)
    }
}
