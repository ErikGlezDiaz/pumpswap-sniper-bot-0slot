use anyhow::Result;
use log::{debug, error, info, warn};
use metrics::{counter, gauge, histogram, register_counter, register_gauge, register_histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::config::Config;

pub struct Monitoring {
    config: Arc<RwLock<Config>>,
    metrics_server: Option<tokio::task::JoinHandle<()>>,
}

// Metrics
lazy_static::lazy_static! {
    static ref TRADES_EXECUTED: metrics::Counter = register_counter!("trades_executed_total", "Total number of trades executed");
    static ref TRADES_SUCCESSFUL: metrics::Counter = register_counter!("trades_successful_total", "Total number of successful trades");
    static ref TRADES_FAILED: metrics::Counter = register_counter!("trades_failed_total", "Total number of failed trades");
    static ref MEV_OPPORTUNITIES: metrics::Counter = register_counter!("mev_opportunities_total", "Total number of MEV opportunities detected");
    static ref MEV_EXECUTED: metrics::Counter = register_counter!("mev_executed_total", "Total number of MEV strategies executed");
    static ref PROFIT_EARNED: metrics::Counter = register_counter!("profit_earned_sol", "Total profit earned in SOL");
    static ref GAS_SPENT: metrics::Counter = register_counter!("gas_spent_lamports", "Total gas spent in lamports");
    static ref ACTIVE_TRADES: metrics::Gauge = register_gauge!("active_trades", "Number of currently active trades");
    static ref LATENCY_MS: metrics::Histogram = register_histogram!("execution_latency_ms", "Trade execution latency in milliseconds");
    static ref BUNDLE_CONFIRMATION_TIME: metrics::Histogram = register_histogram!("bundle_confirmation_time_ms", "Bundle confirmation time in milliseconds");
    static ref PRICE_IMPACT: metrics::Histogram = register_histogram!("price_impact_percentage", "Price impact percentage");
    static ref SLIPPAGE: metrics::Histogram = register_histogram!("slippage_percentage", "Slippage percentage");
}

impl Monitoring {
    pub async fn new(config: Arc<RwLock<Config>>) -> Result<Self> {
        let config_guard = config.read().await;
        
        let mut monitoring = Self {
            config,
            metrics_server: None,
        };
        
        // Start metrics server if enabled
        if config_guard.enable_metrics {
            monitoring.start_metrics_server().await?;
        }
        
        Ok(monitoring)
    }
    
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting monitoring system");
        
        // Start system monitoring
        let system_monitor_handle = {
            let config = self.config.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(10));
                loop {
                    interval.tick().await;
                    
                    if let Err(e) = Self::collect_system_metrics(&config).await {
                        error!("Error collecting system metrics: {}", e);
                    }
                }
            })
        };
        
        // Start performance monitoring
        let performance_monitor_handle = {
            let config = self.config.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    
                    if let Err(e) = Self::collect_performance_metrics(&config).await {
                        error!("Error collecting performance metrics: {}", e);
                    }
                }
            })
        };
        
        // Start risk monitoring
        let risk_monitor_handle = {
            let config = self.config.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(1));
                loop {
                    interval.tick().await;
                    
                    if let Err(e) = Self::monitor_risk_metrics(&config).await {
                        error!("Error monitoring risk metrics: {}", e);
                    }
                }
            })
        };
        
        info!("Monitoring system started successfully");
        
        // Wait for monitoring tasks
        tokio::select! {
            _ = system_monitor_handle => {
                info!("System monitor completed");
            }
            _ = performance_monitor_handle => {
                info!("Performance monitor completed");
            }
            _ = risk_monitor_handle => {
                info!("Risk monitor completed");
            }
        }
        
        Ok(())
    }
    
    async fn start_metrics_server(&mut self) -> Result<()> {
        let config_guard = self.config.read().await;
        let port = config_guard.metrics_port;
        drop(config_guard);
        
        let builder = PrometheusBuilder::new();
        let handle = builder
            .with_http_listener(([0, 0, 0, 0], port))
            .install()?;
        
        let metrics_server = tokio::spawn(async move {
            info!("Metrics server started on port {}", port);
            // Keep the server running
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
        
        self.metrics_server = Some(metrics_server);
        Ok(())
    }
    
    async fn collect_system_metrics(config: &Arc<RwLock<Config>>) -> Result<()> {
        // Collect system resource usage
        let memory_usage = Self::get_memory_usage().await?;
        let cpu_usage = Self::get_cpu_usage().await?;
        let network_usage = Self::get_network_usage().await?;
        
        // Update metrics
        gauge!("system_memory_usage_bytes", memory_usage);
        gauge!("system_cpu_usage_percentage", cpu_usage);
        gauge!("system_network_usage_bytes", network_usage);
        
        debug!("System metrics collected: memory={}MB, cpu={}%, network={}MB", 
               memory_usage / 1024 / 1024, cpu_usage, network_usage / 1024 / 1024);
        
        Ok(())
    }
    
    async fn collect_performance_metrics(config: &Arc<RwLock<Config>>) -> Result<()> {
        let config_guard = config.read().await;
        
        // Collect performance metrics
        let active_connections = Self::get_active_connections().await?;
        let queue_size = Self::get_queue_size().await?;
        let error_rate = Self::get_error_rate().await?;
        
        // Update metrics
        gauge!("active_connections", active_connections as f64);
        gauge!("queue_size", queue_size as f64);
        gauge!("error_rate_percentage", error_rate);
        
        debug!("Performance metrics collected: connections={}, queue={}, error_rate={}%", 
               active_connections, queue_size, error_rate);
        
        Ok(())
    }
    
    async fn monitor_risk_metrics(config: &Arc<RwLock<Config>>) -> Result<()> {
        let config_guard = config.read().await;
        
        // Check risk limits
        let daily_pnl = Self::get_daily_pnl().await?;
        let max_daily_loss = config_guard.max_daily_loss;
        let max_position_size = config_guard.max_position_size;
        
        // Update metrics
        gauge!("daily_pnl_sol", daily_pnl);
        gauge!("max_daily_loss_sol", max_daily_loss);
        gauge!("max_position_size_sol", max_position_size);
        
        // Check if we're approaching risk limits
        if daily_pnl < -max_daily_loss * 0.8 {
            warn!("Approaching daily loss limit: {} SOL (limit: {} SOL)", daily_pnl, max_daily_loss);
        }
        
        if daily_pnl < -max_daily_loss {
            error!("Daily loss limit exceeded: {} SOL (limit: {} SOL)", daily_pnl, max_daily_loss);
            // In a real implementation, this would trigger risk management actions
        }
        
        Ok(())
    }
    
    // Metric recording functions
    pub fn record_trade_executed() {
        counter!(TRADES_EXECUTED, 1.0);
    }
    
    pub fn record_trade_successful() {
        counter!(TRADES_SUCCESSFUL, 1.0);
    }
    
    pub fn record_trade_failed() {
        counter!(TRADES_FAILED, 1.0);
    }
    
    pub fn record_mev_opportunity() {
        counter!(MEV_OPPORTUNITIES, 1.0);
    }
    
    pub fn record_mev_executed() {
        counter!(MEV_EXECUTED, 1.0);
    }
    
    pub fn record_profit_earned(profit_sol: f64) {
        counter!(PROFIT_EARNED, profit_sol);
    }
    
    pub fn record_gas_spent(gas_lamports: u64) {
        counter!(GAS_SPENT, gas_lamports as f64);
    }
    
    pub fn update_active_trades(count: usize) {
        gauge!(ACTIVE_TRADES, count as f64);
    }
    
    pub fn record_execution_latency(latency_ms: f64) {
        histogram!(LATENCY_MS, latency_ms);
    }
    
    pub fn record_bundle_confirmation_time(time_ms: f64) {
        histogram!(BUNDLE_CONFIRMATION_TIME, time_ms);
    }
    
    pub fn record_price_impact(impact_percentage: f64) {
        histogram!(PRICE_IMPACT, impact_percentage);
    }
    
    pub fn record_slippage(slippage_percentage: f64) {
        histogram!(SLIPPAGE, slippage_percentage);
    }
    
    // System monitoring helper functions
    async fn get_memory_usage() -> Result<u64> {
        // This would get actual memory usage
        // For now, return a simulated value
        Ok(1024 * 1024 * 100) // 100MB
    }
    
    async fn get_cpu_usage() -> Result<f64> {
        // This would get actual CPU usage
        // For now, return a simulated value
        Ok(25.0) // 25%
    }
    
    async fn get_network_usage() -> Result<u64> {
        // This would get actual network usage
        // For now, return a simulated value
        Ok(1024 * 1024 * 10) // 10MB
    }
    
    async fn get_active_connections() -> Result<usize> {
        // This would get actual connection count
        // For now, return a simulated value
        Ok(5)
    }
    
    async fn get_queue_size() -> Result<usize> {
        // This would get actual queue size
        // For now, return a simulated value
        Ok(0)
    }
    
    async fn get_error_rate() -> Result<f64> {
        // This would calculate actual error rate
        // For now, return a simulated value
        Ok(0.1) // 0.1%
    }
    
    async fn get_daily_pnl() -> Result<f64> {
        // This would get actual daily P&L
        // For now, return a simulated value
        Ok(5.5) // 5.5 SOL profit
    }
}

pub struct TradeLogger {
    config: Arc<RwLock<Config>>,
}

impl TradeLogger {
    pub fn new(config: Arc<RwLock<Config>>) -> Self {
        Self { config }
    }
    
    pub fn log_trade_start(&self, token_address: &str, amount: u64, strategy: &str) {
        info!("Trade started: token={}, amount={} lamports, strategy={}", 
              token_address, amount, strategy);
        
        Monitoring::record_trade_executed();
    }
    
    pub fn log_trade_success(&self, token_address: &str, profit: f64, gas_used: u64, latency_ms: f64) {
        info!("Trade successful: token={}, profit={} SOL, gas={} lamports, latency={}ms", 
              token_address, profit, gas_used, latency_ms);
        
        Monitoring::record_trade_successful();
        Monitoring::record_profit_earned(profit);
        Monitoring::record_gas_spent(gas_used);
        Monitoring::record_execution_latency(latency_ms);
    }
    
    pub fn log_trade_failure(&self, token_address: &str, error: &str, gas_used: u64) {
        warn!("Trade failed: token={}, error={}, gas={} lamports", 
              token_address, error, gas_used);
        
        Monitoring::record_trade_failed();
        Monitoring::record_gas_spent(gas_used);
    }
    
    pub fn log_mev_opportunity(&self, strategy: &str, token_address: &str, expected_profit: f64) {
        info!("MEV opportunity: strategy={}, token={}, expected_profit={} SOL", 
              strategy, token_address, expected_profit);
        
        Monitoring::record_mev_opportunity();
    }
    
    pub fn log_mev_execution(&self, strategy: &str, token_address: &str, actual_profit: f64) {
        info!("MEV executed: strategy={}, token={}, actual_profit={} SOL", 
              strategy, token_address, actual_profit);
        
        Monitoring::record_mev_executed();
        Monitoring::record_profit_earned(actual_profit);
    }
    
    pub fn log_bundle_submission(&self, bundle_id: &str, transaction_count: usize) {
        info!("Bundle submitted: id={}, transactions={}", bundle_id, transaction_count);
    }
    
    pub fn log_bundle_confirmation(&self, bundle_id: &str, confirmation_time_ms: f64) {
        info!("Bundle confirmed: id={}, confirmation_time={}ms", bundle_id, confirmation_time_ms);
        
        Monitoring::record_bundle_confirmation_time(confirmation_time_ms);
    }
    
    pub fn log_price_impact(&self, token_address: &str, impact_percentage: f64) {
        debug!("Price impact: token={}, impact={}%", token_address, impact_percentage);
        
        Monitoring::record_price_impact(impact_percentage);
    }
    
    pub fn log_slippage(&self, token_address: &str, slippage_percentage: f64) {
        debug!("Slippage: token={}, slippage={}%", token_address, slippage_percentage);
        
        Monitoring::record_slippage(slippage_percentage);
    }
}
