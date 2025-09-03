use anyhow::Result;
use log::{debug, error, info, warn};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    hash::Hash,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::config::Config;

pub struct JitoClient {
    rpc_client: RpcClient,
    config: Arc<RwLock<Config>>,
    tip_account: Pubkey,
}

#[derive(Debug, Clone)]
pub struct BundleTransaction {
    pub transaction: Transaction,
    pub priority_fee: u64,
    pub tip_amount: u64,
}

#[derive(Debug, Clone)]
pub struct Bundle {
    pub transactions: Vec<BundleTransaction>,
    pub bundle_id: String,
    pub created_at: u64,
}

impl JitoClient {
    pub fn new(config: Arc<RwLock<Config>>) -> Result<Self> {
        let config_guard = config.read().unwrap();
        let rpc_client = RpcClient::new_with_commitment(
            config_guard.solana_rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );
        
        let tip_account = config_guard.jito_tip_account.parse()?;
        drop(config_guard);
        
        Ok(Self {
            rpc_client,
            config,
            tip_account,
        })
    }
    
    pub async fn create_bundle(&self, transactions: Vec<Transaction>, keypair: &Keypair) -> Result<Bundle> {
        let mut bundle_transactions = Vec::new();
        let config_guard = self.config.read().await;
        
        for transaction in transactions {
            // Calculate priority fee based on current network conditions
            let priority_fee = self.calculate_priority_fee().await?;
            
            // Create tip transaction
            let tip_amount = config_guard.jito_tip_amount;
            let tip_transaction = self.create_tip_transaction(keypair, tip_amount).await?;
            
            bundle_transactions.push(BundleTransaction {
                transaction,
                priority_fee,
                tip_amount,
            });
            
            bundle_transactions.push(BundleTransaction {
                transaction: tip_transaction,
                priority_fee: 0,
                tip_amount: 0,
            });
        }
        
        let bundle_id = self.generate_bundle_id();
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        drop(config_guard);
        
        Ok(Bundle {
            transactions: bundle_transactions,
            bundle_id,
            created_at,
        })
    }
    
    pub async fn submit_bundle(&self, bundle: &Bundle) -> Result<String> {
        info!("Submitting bundle {} with {} transactions", bundle.bundle_id, bundle.transactions.len());
        
        // Convert bundle transactions to Solana transactions
        let transactions: Vec<Transaction> = bundle.transactions
            .iter()
            .map(|bt| bt.transaction.clone())
            .collect();
        
        // Submit bundle to Jito
        let bundle_id = self.rpc_client
            .send_bundle(&transactions)
            .await?;
        
        info!("Bundle submitted successfully: {}", bundle_id);
        Ok(bundle_id)
    }
    
    pub async fn wait_for_bundle_confirmation(&self, bundle_id: &str, timeout: Duration) -> Result<bool> {
        let start_time = SystemTime::now();
        
        info!("Waiting for bundle confirmation: {}", bundle_id);
        
        while start_time.elapsed()? < timeout {
            // Check bundle status
            match self.get_bundle_status(bundle_id).await {
                Ok(status) => {
                    match status {
                        BundleStatus::Confirmed => {
                            info!("Bundle {} confirmed successfully", bundle_id);
                            return Ok(true);
                        }
                        BundleStatus::Failed => {
                            warn!("Bundle {} failed", bundle_id);
                            return Ok(false);
                        }
                        BundleStatus::Pending => {
                            debug!("Bundle {} still pending", bundle_id);
                        }
                    }
                }
                Err(e) => {
                    debug!("Error checking bundle status: {}", e);
                }
            }
            
            // Wait before next check
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        warn!("Bundle {} confirmation timeout", bundle_id);
        Ok(false)
    }
    
    async fn calculate_priority_fee(&self) -> Result<u64> {
        // Get recent priority fee data
        let recent_fees = self.rpc_client
            .get_recent_prioritization_fees(&[])
            .await?;
        
        if recent_fees.is_empty() {
            return Ok(100000); // Default priority fee
        }
        
        // Calculate average priority fee with multiplier
        let config_guard = self.config.read().await;
        let multiplier = config_guard.priority_fee_multiplier;
        drop(config_guard);
        
        let avg_fee = recent_fees.iter().map(|f| f.prioritization_fee).sum::<u64>() / recent_fees.len() as u64;
        let adjusted_fee = (avg_fee as f64 * multiplier) as u64;
        
        Ok(adjusted_fee.max(100000)) // Minimum 0.0001 SOL
    }
    
    async fn create_tip_transaction(&self, keypair: &Keypair, tip_amount: u64) -> Result<Transaction> {
        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        
        // Create tip instruction
        let tip_instruction = solana_sdk::system_instruction::transfer(
            &keypair.pubkey(),
            &self.tip_account,
            tip_amount,
        );
        
        let message = Message::new(&[tip_instruction], Some(&keypair.pubkey()));
        let transaction = Transaction::new(&[keypair], message, recent_blockhash);
        
        Ok(transaction)
    }
    
    fn generate_bundle_id(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; 16] = rng.gen();
        hex::encode(random_bytes)
    }
    
    async fn get_bundle_status(&self, bundle_id: &str) -> Result<BundleStatus> {
        // This would typically involve checking Jito's bundle status endpoint
        // For now, we'll simulate the status check
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Simulate random status for demo purposes
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let status = match rng.gen_range(0..3) {
            0 => BundleStatus::Pending,
            1 => BundleStatus::Confirmed,
            _ => BundleStatus::Failed,
        };
        
        Ok(status)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BundleStatus {
    Pending,
    Confirmed,
    Failed,
}

pub struct BundleManager {
    jito_client: JitoClient,
    pending_bundles: std::collections::HashMap<String, Bundle>,
    config: Arc<RwLock<Config>>,
}

impl BundleManager {
    pub fn new(config: Arc<RwLock<Config>>) -> Result<Self> {
        let jito_client = JitoClient::new(config.clone())?;
        
        Ok(Self {
            jito_client,
            pending_bundles: std::collections::HashMap::new(),
            config,
        })
    }
    
    pub async fn submit_transaction_bundle(&mut self, transactions: Vec<Transaction>, keypair: &Keypair) -> Result<String> {
        // Create bundle
        let bundle = self.jito_client.create_bundle(transactions, keypair).await?;
        let bundle_id = bundle.bundle_id.clone();
        
        // Store bundle
        self.pending_bundles.insert(bundle_id.clone(), bundle);
        
        // Submit bundle
        let submitted_id = self.jito_client.submit_bundle(&self.pending_bundles[&bundle_id]).await?;
        
        // Start confirmation monitoring
        let config_guard = self.config.read().await;
        let timeout = Duration::from_millis(config_guard.bundle_timeout);
        drop(config_guard);
        
        let jito_client = self.jito_client.clone();
        let bundle_id_clone = bundle_id.clone();
        tokio::spawn(async move {
            let confirmed = jito_client.wait_for_bundle_confirmation(&submitted_id, timeout).await;
            match confirmed {
                Ok(true) => info!("Bundle {} confirmed", submitted_id),
                Ok(false) => warn!("Bundle {} failed or timed out", submitted_id),
                Err(e) => error!("Error waiting for bundle {}: {}", submitted_id, e),
            }
        });
        
        Ok(submitted_id)
    }
    
    pub async fn get_bundle_status(&self, bundle_id: &str) -> Option<BundleStatus> {
        // Check if bundle is still pending
        if self.pending_bundles.contains_key(bundle_id) {
            Some(BundleStatus::Pending)
        } else {
            // Bundle has been processed
            Some(BundleStatus::Confirmed)
        }
    }
    
    pub fn cleanup_completed_bundles(&mut self) {
        // Remove bundles that are no longer pending
        self.pending_bundles.retain(|_, bundle| {
            let elapsed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() - bundle.created_at;
            
            elapsed < 300 // Keep bundles for 5 minutes
        });
    }
}

// Clone implementation for JitoClient
impl Clone for JitoClient {
    fn clone(&self) -> Self {
        Self {
            rpc_client: self.rpc_client.clone(),
            config: self.config.clone(),
            tip_account: self.tip_account,
        }
    }
}
