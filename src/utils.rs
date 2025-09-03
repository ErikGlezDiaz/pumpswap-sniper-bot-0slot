use anyhow::Result;
use log::{debug, info, warn};
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    transaction::Transaction,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn generate_trade_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_bytes: [u8; 16] = rng.gen();
    format!("trade_{}", hex::encode(random_bytes))
}

pub fn generate_bundle_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_bytes: [u8; 16] = rng.gen();
    format!("bundle_{}", hex::encode(random_bytes))
}

pub fn calculate_price_impact(
    input_amount: u64,
    output_amount: u64,
    pool_reserves: (u64, u64),
) -> f64 {
    let (reserve_in, reserve_out) = pool_reserves;
    
    if reserve_in == 0 || reserve_out == 0 {
        return 0.0;
    }
    
    // Calculate price impact using constant product formula
    let new_reserve_in = reserve_in + input_amount;
    let new_reserve_out = reserve_out - output_amount;
    
    let price_before = reserve_out as f64 / reserve_in as f64;
    let price_after = new_reserve_out as f64 / new_reserve_in as f64;
    
    let price_impact = (price_before - price_after) / price_before * 100.0;
    price_impact.max(0.0)
}

pub fn calculate_slippage(
    expected_output: u64,
    actual_output: u64,
) -> f64 {
    if expected_output == 0 {
        return 0.0;
    }
    
    let slippage = (expected_output as f64 - actual_output as f64) / expected_output as f64 * 100.0;
    slippage.max(0.0)
}

pub fn calculate_optimal_gas_price(
    base_gas_price: u64,
    network_congestion: f64,
    priority_multiplier: f64,
) -> u64 {
    let congestion_multiplier = 1.0 + network_congestion;
    let adjusted_price = (base_gas_price as f64 * congestion_multiplier * priority_multiplier) as u64;
    
    // Ensure minimum gas price
    adjusted_price.max(5000) // 0.000005 SOL
}

pub fn validate_token_address(address: &str) -> Result<()> {
    if address.len() != 44 {
        return Err(anyhow::anyhow!("Invalid token address length: {}", address.len()));
    }
    
    // Try to parse as Pubkey
    address.parse::<Pubkey>()?;
    
    Ok(())
}

pub fn validate_pool_address(address: &str) -> Result<()> {
    if address.len() != 44 {
        return Err(anyhow::anyhow!("Invalid pool address length: {}", address.len()));
    }
    
    // Try to parse as Pubkey
    address.parse::<Pubkey>()?;
    
    Ok(())
}

pub fn format_amount(amount: u64, decimals: u8) -> String {
    let divisor = 10_u64.pow(decimals as u32);
    let whole = amount / divisor;
    let fraction = amount % divisor;
    
    if fraction == 0 {
        format!("{}", whole)
    } else {
        format!("{}.{:0width$}", whole, fraction, width = decimals as usize)
    }
}

pub fn parse_amount(amount_str: &str, decimals: u8) -> Result<u64> {
    let parts: Vec<&str> = amount_str.split('.').collect();
    
    match parts.len() {
        1 => {
            let whole: u64 = parts[0].parse()?;
            Ok(whole * 10_u64.pow(decimals as u32))
        }
        2 => {
            let whole: u64 = parts[0].parse()?;
            let fraction_str = parts[1];
            
            if fraction_str.len() > decimals as usize {
                return Err(anyhow::anyhow!("Too many decimal places"));
            }
            
            let fraction: u64 = format!("{:0<width$}", fraction_str, width = decimals as usize).parse()?;
            
            Ok(whole * 10_u64.pow(decimals as u32) + fraction)
        }
        _ => Err(anyhow::anyhow!("Invalid amount format")),
    }
}

pub fn calculate_profit_margin(
    buy_price: f64,
    sell_price: f64,
    fees: f64,
) -> f64 {
    if buy_price == 0.0 {
        return 0.0;
    }
    
    let gross_profit = sell_price - buy_price;
    let net_profit = gross_profit - fees;
    let profit_margin = (net_profit / buy_price) * 100.0;
    
    profit_margin
}

pub fn estimate_transaction_fee(
    transaction_size: usize,
    gas_price: u64,
) -> u64 {
    // Base transaction fee
    let base_fee = 5000; // 0.000005 SOL
    
    // Fee per byte
    let fee_per_byte = 100; // 0.0000001 SOL per byte
    
    // Priority fee
    let priority_fee = gas_price;
    
    let total_fee = base_fee + (transaction_size as u64 * fee_per_byte) + priority_fee;
    
    total_fee
}

pub fn calculate_optimal_slippage(
    liquidity: u64,
    trade_size: u64,
    volatility: f64,
) -> f64 {
    // Base slippage calculation
    let liquidity_ratio = trade_size as f64 / liquidity as f64;
    let base_slippage = liquidity_ratio * 100.0;
    
    // Adjust for volatility
    let volatility_adjustment = volatility * 0.5;
    
    // Add safety margin
    let safety_margin = 0.5;
    
    let optimal_slippage = base_slippage + volatility_adjustment + safety_margin;
    
    // Cap at reasonable maximum
    optimal_slippage.min(10.0) // 10% maximum
}

pub fn is_profitable_trade(
    expected_profit: f64,
    gas_cost: u64,
    risk_factor: f64,
) -> bool {
    let gas_cost_sol = gas_cost as f64 / 1e9; // Convert lamports to SOL
    let risk_adjusted_profit = expected_profit * (1.0 - risk_factor);
    
    risk_adjusted_profit > gas_cost_sol * 2.0 // At least 2x gas cost
}

pub fn calculate_position_size(
    account_balance: u64,
    risk_percentage: f64,
    token_price: f64,
) -> u64 {
    let max_risk_amount = (account_balance as f64 * risk_percentage / 100.0) as u64;
    let position_size = (max_risk_amount as f64 / token_price) as u64;
    
    position_size
}

pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

pub fn get_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn get_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn is_within_time_window(
    timestamp: u64,
    window_seconds: u64,
) -> bool {
    let current_time = get_timestamp();
    current_time - timestamp <= window_seconds
}

pub fn calculate_confidence_score(
    liquidity: u64,
    volume: u64,
    price_stability: f64,
    time_since_listing: u64,
) -> f64 {
    // Liquidity score (0-0.4)
    let liquidity_score = (liquidity as f64 / 1e9).min(100.0) / 100.0 * 0.4;
    
    // Volume score (0-0.3)
    let volume_score = (volume as f64 / 1e6).min(1000.0) / 1000.0 * 0.3;
    
    // Price stability score (0-0.2)
    let stability_score = (1.0 - price_stability).max(0.0) * 0.2;
    
    // Time score (0-0.1)
    let time_score = (time_since_listing as f64 / 3600.0).min(24.0) / 24.0 * 0.1;
    
    liquidity_score + volume_score + stability_score + time_score
}

pub fn validate_transaction(transaction: &Transaction) -> Result<()> {
    // Check transaction size
    let serialized = bincode::serialize(transaction)?;
    if serialized.len() > 1232 {
        return Err(anyhow::anyhow!("Transaction too large: {} bytes", serialized.len()));
    }
    
    // Check for required signatures
    if transaction.signatures.is_empty() {
        return Err(anyhow::anyhow!("Transaction has no signatures"));
    }
    
    // Check message
    if transaction.message.instructions.is_empty() {
        return Err(anyhow::anyhow!("Transaction has no instructions"));
    }
    
    Ok(())
}

pub fn estimate_execution_time(
    network_congestion: f64,
    priority_fee: u64,
    base_time_ms: u64,
) -> u64 {
    let congestion_factor = 1.0 + network_congestion;
    let priority_factor = if priority_fee > 100000 { 0.5 } else { 1.0 };
    
    let estimated_time = (base_time_ms as f64 * congestion_factor * priority_factor) as u64;
    
    estimated_time.max(100) // Minimum 100ms
}

pub fn calculate_risk_score(
    volatility: f64,
    liquidity: u64,
    market_cap: u64,
    time_since_listing: u64,
) -> f64 {
    // Volatility risk (0-0.4)
    let volatility_risk = volatility.min(1.0) * 0.4;
    
    // Liquidity risk (0-0.3)
    let liquidity_risk = (1.0 - (liquidity as f64 / 1e9).min(100.0) / 100.0) * 0.3;
    
    // Market cap risk (0-0.2)
    let market_cap_risk = (1.0 - (market_cap as f64 / 1e6).min(1000.0) / 1000.0) * 0.2;
    
    // Time risk (0-0.1)
    let time_risk = (1.0 - (time_since_listing as f64 / 3600.0).min(24.0) / 24.0) * 0.1;
    
    volatility_risk + liquidity_risk + market_cap_risk + time_risk
}

pub fn should_execute_trade(
    expected_profit: f64,
    risk_score: f64,
    confidence_score: f64,
    min_profit_threshold: f64,
    max_risk_threshold: f64,
    min_confidence_threshold: f64,
) -> bool {
    expected_profit >= min_profit_threshold
        && risk_score <= max_risk_threshold
        && confidence_score >= min_confidence_threshold
}

pub fn log_trade_metrics(
    token_address: &str,
    strategy: &str,
    expected_profit: f64,
    actual_profit: f64,
    gas_used: u64,
    execution_time_ms: u64,
    slippage: f64,
    price_impact: f64,
) {
    info!("Trade metrics: token={}, strategy={}, expected_profit={} SOL, actual_profit={} SOL, gas={} lamports, time={}ms, slippage={}%, impact={}%",
          token_address, strategy, expected_profit, actual_profit, gas_used, execution_time_ms, slippage, price_impact);
}
