use anyhow::Result;
use log::{debug, error, info, warn};
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::{Channel, Endpoint};
use tonic::Request;

use crate::config::Config;
use crate::proto::pumpswap::pumpswap_service_client::PumpSwapServiceClient;
use crate::proto::pumpswap::*;

pub struct PumpSwapGrpcClient {
    client: PumpSwapServiceClient<Channel>,
    config: Arc<RwLock<Config>>,
}

impl PumpSwapGrpcClient {
    pub async fn new(config: Arc<RwLock<Config>>) -> Result<Self> {
        let config_guard = config.read().await;
        let endpoint = Endpoint::from_shared(config_guard.pumpswap_grpc_url.clone())?;
        drop(config_guard);
        
        let channel = endpoint.connect().await?;
        let mut client = PumpSwapServiceClient::new(channel);
        
        // Add authentication if API key is provided
        if let Some(api_key) = &config.read().await.pumpswap_api_key {
            client = client.add_header("authorization", format!("Bearer {}", api_key))?;
        }
        
        Ok(Self { client, config })
    }
    
    pub async fn stream_new_listings(&mut self) -> Result<tonic::Streaming<TokenListing>> {
        let request = StreamRequest {
            token_addresses: vec![], // Empty means all tokens
            include_metadata: true,
            max_results: 1000,
        };
        
        let response = self.client
            .stream_new_listings(Request::new(request))
            .await?;
        
        Ok(response.into_inner())
    }
    
    pub async fn get_pool_info(&mut self, pool_address: &str) -> Result<PoolInfo> {
        let request = PoolInfoRequest {
            pool_address: pool_address.to_string(),
            token_address: String::new(),
        };
        
        let response = self.client
            .get_pool_info(Request::new(request))
            .await?;
        
        Ok(response.into_inner())
    }
    
    pub async fn get_token_metadata(&mut self, token_address: &str) -> Result<TokenMetadata> {
        let request = TokenMetadataRequest {
            token_address: token_address.to_string(),
        };
        
        let response = self.client
            .get_token_metadata(Request::new(request))
            .await?;
        
        Ok(response.into_inner())
    }
    
    pub async fn get_price_info(&mut self, token_address: &str) -> Result<PriceInfo> {
        let request = PriceInfoRequest {
            token_address: token_address.to_string(),
            pool_address: String::new(),
        };
        
        let response = self.client
            .get_price_info(Request::new(request))
            .await?;
        
        Ok(response.into_inner())
    }
    
    pub async fn stream_price_updates(&mut self, token_addresses: Vec<String>) -> Result<tonic::Streaming<PriceUpdate>> {
        let request = PriceStreamRequest {
            token_addresses,
            update_interval_ms: 100, // 100ms updates
        };
        
        let response = self.client
            .stream_price_updates(Request::new(request))
            .await?;
        
        Ok(response.into_inner())
    }
    
    pub async fn get_mev_opportunities(&mut self, token_addresses: Vec<String>) -> Result<MEVOpportunities> {
        let config_guard = self.config.read().await;
        let request = MEVRequest {
            token_addresses,
            min_liquidity: (config_guard.min_liquidity * 1e9) as u64, // Convert SOL to lamports
            max_slippage: config_guard.max_slippage,
            max_gas_price: config_guard.max_gas_price,
        };
        drop(config_guard);
        
        let response = self.client
            .get_mev_opportunities(Request::new(request))
            .await?;
        
        Ok(response.into_inner())
    }
    
    pub async fn submit_transaction(&mut self, transaction_data: &str) -> Result<TransactionResponse> {
        let config_guard = self.config.read().await;
        let request = TransactionRequest {
            transaction_data: transaction_data.to_string(),
            max_gas_price: config_guard.max_gas_price,
            deadline: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() + config_guard.transaction_timeout,
            confirmation_service: config_guard.confirmation_service.clone(),
            priority_fee: true,
        };
        drop(config_guard);
        
        let response = self.client
            .submit_transaction(Request::new(request))
            .await?;
        
        Ok(response.into_inner())
    }
}

pub struct TokenListingStream {
    client: PumpSwapGrpcClient,
    target_tokens: Vec<String>,
}

impl TokenListingStream {
    pub fn new(config: Arc<RwLock<Config>>) -> Self {
        Self {
            client: PumpSwapGrpcClient::new(config.clone()).await.unwrap(),
            target_tokens: config.read().await.target_tokens.clone(),
        }
    }
    
    pub async fn start_streaming<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(TokenListing) -> Result<bool>, // Return false to stop streaming
    {
        let mut stream = self.client.stream_new_listings().await?;
        
        info!("Started streaming new token listings");
        
        while let Some(listing) = stream.message().await? {
            debug!("Received new listing: {} ({})", listing.token_symbol, listing.token_address);
            
            // Check if this token is in our target list or if we're monitoring all tokens
            if self.target_tokens.is_empty() || self.target_tokens.contains(&listing.token_address) {
                info!("Processing new listing: {} ({})", listing.token_symbol, listing.token_address);
                
                // Call the callback function
                match callback(listing) {
                    Ok(continue_streaming) => {
                        if !continue_streaming {
                            info!("Streaming stopped by callback");
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error in listing callback: {}", e);
                        // Continue streaming despite errors
                    }
                }
            }
        }
        
        info!("Token listing stream ended");
        Ok(())
    }
}

pub struct PriceUpdateStream {
    client: PumpSwapGrpcClient,
    token_addresses: Vec<String>,
}

impl PriceUpdateStream {
    pub async fn new(config: Arc<RwLock<Config>>, token_addresses: Vec<String>) -> Result<Self> {
        let client = PumpSwapGrpcClient::new(config).await?;
        Ok(Self {
            client,
            token_addresses,
        })
    }
    
    pub async fn start_streaming<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(PriceUpdate) -> Result<bool>, // Return false to stop streaming
    {
        let mut stream = self.client.stream_price_updates(self.token_addresses.clone()).await?;
        
        info!("Started streaming price updates for {} tokens", self.token_addresses.len());
        
        while let Some(update) = stream.message().await? {
            debug!("Received price update for {}: ${:.6}", update.token_address, update.price_usd);
            
            // Call the callback function
            match callback(update) {
                Ok(continue_streaming) => {
                    if !continue_streaming {
                        info!("Price streaming stopped by callback");
                        break;
                    }
                }
                Err(e) => {
                    error!("Error in price update callback: {}", e);
                    // Continue streaming despite errors
                }
            }
        }
        
        info!("Price update stream ended");
        Ok(())
    }
}
