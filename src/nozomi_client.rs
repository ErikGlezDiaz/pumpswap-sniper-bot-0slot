use anyhow::Result;
use log::{debug, error, info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::Keypair,
    transaction::Transaction,
};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NozomiTransaction {
    pub transaction_data: String, // Base64 encoded transaction
    pub priority_fee: u64,
    pub max_retries: u32,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NozomiSubmission {
    pub transactions: Vec<NozomiTransaction>,
    pub submission_id: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NozomiResponse {
    pub success: bool,
    pub submission_id: String,
    pub transaction_ids: Vec<String>,
    pub error_message: Option<String>,
    pub confirmation_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NozomiStatus {
    pub submission_id: String,
    pub status: String, // "pending", "confirmed", "failed"
    pub confirmed_transactions: Vec<String>,
    pub failed_transactions: Vec<String>,
    pub confirmation_time_ms: Option<u64>,
}

pub struct NozomiClient {
    client: Client,
    config: Arc<RwLock<Config>>,
    base_url: String,
}

impl NozomiClient {
    pub fn new(config: Arc<RwLock<Config>>) -> Result<Self> {
        let config_guard = config.read().unwrap();
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        
        let base_url = config_guard.nozomi_url.clone();
        drop(config_guard);
        
        Ok(Self {
            client,
            config,
            base_url,
        })
    }
    
    pub async fn submit_transaction(&self, transaction: &Transaction) -> Result<String> {
        let transaction_data = base64::encode(bincode::serialize(transaction)?);
        
        let nozomi_tx = NozomiTransaction {
            transaction_data,
            priority_fee: self.calculate_priority_fee().await?,
            max_retries: 3,
            timeout_ms: 30000,
        };
        
        let submission = NozomiSubmission {
            transactions: vec![nozomi_tx],
            submission_id: self.generate_submission_id(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        };
        
        let response = self.submit_to_nozomi(&submission).await?;
        
        if response.success {
            info!("Transaction submitted to Nozomi: {}", response.submission_id);
            Ok(response.submission_id)
        } else {
            Err(anyhow::anyhow!("Nozomi submission failed: {:?}", response.error_message))
        }
    }
    
    pub async fn submit_transaction_batch(&self, transactions: Vec<Transaction>) -> Result<String> {
        let mut nozomi_transactions = Vec::new();
        
        for transaction in transactions {
            let transaction_data = base64::encode(bincode::serialize(&transaction)?);
            
            let nozomi_tx = NozomiTransaction {
                transaction_data,
                priority_fee: self.calculate_priority_fee().await?,
                max_retries: 3,
                timeout_ms: 30000,
            };
            
            nozomi_transactions.push(nozomi_tx);
        }
        
        let submission = NozomiSubmission {
            transactions: nozomi_transactions,
            submission_id: self.generate_submission_id(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        };
        
        let response = self.submit_to_nozomi(&submission).await?;
        
        if response.success {
            info!("Batch of {} transactions submitted to Nozomi: {}", 
                  submission.transactions.len(), response.submission_id);
            Ok(response.submission_id)
        } else {
            Err(anyhow::anyhow!("Nozomi batch submission failed: {:?}", response.error_message))
        }
    }
    
    pub async fn wait_for_confirmation(&self, submission_id: &str, timeout: Duration) -> Result<bool> {
        let start_time = SystemTime::now();
        
        info!("Waiting for Nozomi confirmation: {}", submission_id);
        
        while start_time.elapsed()? < timeout {
            match self.get_submission_status(submission_id).await {
                Ok(status) => {
                    match status.status.as_str() {
                        "confirmed" => {
                            info!("Nozomi submission {} confirmed in {}ms", 
                                  submission_id, 
                                  status.confirmation_time_ms.unwrap_or(0));
                            return Ok(true);
                        }
                        "failed" => {
                            warn!("Nozomi submission {} failed", submission_id);
                            return Ok(false);
                        }
                        "pending" => {
                            debug!("Nozomi submission {} still pending", submission_id);
                        }
                        _ => {
                            debug!("Unknown Nozomi status: {}", status.status);
                        }
                    }
                }
                Err(e) => {
                    debug!("Error checking Nozomi status: {}", e);
                }
            }
            
            // Wait before next check
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        warn!("Nozomi confirmation timeout for submission: {}", submission_id);
        Ok(false)
    }
    
    async fn submit_to_nozomi(&self, submission: &NozomiSubmission) -> Result<NozomiResponse> {
        let url = format!("{}/api/v1/submit", self.base_url);
        
        let mut request = self.client.post(&url).json(submission);
        
        // Add API key if configured
        if let Some(api_key) = &self.config.read().await.nozomi_api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = request.send().await?;
        
        if response.status().is_success() {
            let nozomi_response: NozomiResponse = response.json().await?;
            Ok(nozomi_response)
        } else {
            let error_text = response.text().await?;
            Err(anyhow::anyhow!("Nozomi API error: {}", error_text))
        }
    }
    
    async fn get_submission_status(&self, submission_id: &str) -> Result<NozomiStatus> {
        let url = format!("{}/api/v1/status/{}", self.base_url, submission_id);
        
        let mut request = self.client.get(&url);
        
        // Add API key if configured
        if let Some(api_key) = &self.config.read().await.nozomi_api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = request.send().await?;
        
        if response.status().is_success() {
            let status: NozomiStatus = response.json().await?;
            Ok(status)
        } else {
            let error_text = response.text().await?;
            Err(anyhow::anyhow!("Nozomi status API error: {}", error_text))
        }
    }
    
    async fn calculate_priority_fee(&self) -> Result<u64> {
        // Nozomi handles priority fee optimization internally
        // We just need to provide a reasonable base fee
        let config_guard = self.config.read().await;
        let base_fee = config_guard.max_gas_price / 10; // 10% of max gas price
        drop(config_guard);
        
        Ok(base_fee.max(50000)) // Minimum 0.00005 SOL
    }
    
    fn generate_submission_id(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; 16] = rng.gen();
        format!("nozomi_{}", hex::encode(random_bytes))
    }
}

pub struct NozomiManager {
    client: NozomiClient,
    pending_submissions: std::collections::HashMap<String, NozomiSubmission>,
    config: Arc<RwLock<Config>>,
}

impl NozomiManager {
    pub fn new(config: Arc<RwLock<Config>>) -> Result<Self> {
        let client = NozomiClient::new(config.clone())?;
        
        Ok(Self {
            client,
            pending_submissions: std::collections::HashMap::new(),
            config,
        })
    }
    
    pub async fn submit_transaction(&mut self, transaction: &Transaction) -> Result<String> {
        let submission_id = self.client.submit_transaction(transaction).await?;
        
        // Store submission for tracking
        let submission = NozomiSubmission {
            transactions: vec![], // We don't need to store the actual transaction data
            submission_id: submission_id.clone(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        };
        
        self.pending_submissions.insert(submission_id.clone(), submission);
        
        // Start confirmation monitoring
        let config_guard = self.config.read().await;
        let timeout = Duration::from_millis(config_guard.transaction_timeout * 1000);
        drop(config_guard);
        
        let client = self.client.clone();
        let submission_id_clone = submission_id.clone();
        tokio::spawn(async move {
            let confirmed = client.wait_for_confirmation(&submission_id_clone, timeout).await;
            match confirmed {
                Ok(true) => info!("Nozomi submission {} confirmed", submission_id_clone),
                Ok(false) => warn!("Nozomi submission {} failed or timed out", submission_id_clone),
                Err(e) => error!("Error waiting for Nozomi submission {}: {}", submission_id_clone, e),
            }
        });
        
        Ok(submission_id)
    }
    
    pub async fn submit_transaction_batch(&mut self, transactions: Vec<Transaction>) -> Result<String> {
        let submission_id = self.client.submit_transaction_batch(transactions).await?;
        
        // Store submission for tracking
        let submission = NozomiSubmission {
            transactions: vec![], // We don't need to store the actual transaction data
            submission_id: submission_id.clone(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        };
        
        self.pending_submissions.insert(submission_id.clone(), submission);
        
        // Start confirmation monitoring
        let config_guard = self.config.read().await;
        let timeout = Duration::from_millis(config_guard.transaction_timeout * 1000);
        drop(config_guard);
        
        let client = self.client.clone();
        let submission_id_clone = submission_id.clone();
        tokio::spawn(async move {
            let confirmed = client.wait_for_confirmation(&submission_id_clone, timeout).await;
            match confirmed {
                Ok(true) => info!("Nozomi batch submission {} confirmed", submission_id_clone),
                Ok(false) => warn!("Nozomi batch submission {} failed or timed out", submission_id_clone),
                Err(e) => error!("Error waiting for Nozomi batch submission {}: {}", submission_id_clone, e),
            }
        });
        
        Ok(submission_id)
    }
    
    pub async fn get_submission_status(&self, submission_id: &str) -> Result<NozomiStatus> {
        self.client.get_submission_status(submission_id).await
    }
    
    pub fn cleanup_completed_submissions(&mut self) {
        // Remove submissions that are older than 10 minutes
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - 600; // 10 minutes
        
        self.pending_submissions.retain(|_, submission| {
            submission.created_at > cutoff_time
        });
    }
}

// Clone implementation for NozomiClient
impl Clone for NozomiClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: self.config.clone(),
            base_url: self.base_url.clone(),
        }
    }
}
