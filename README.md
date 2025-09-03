# PumpSwap 0-Slot Sniper Bot

A high-performance MEV bot for PumpSwap that uses gRPC for real-time data streaming and Jito/Nozomi confirmation services for 0-slot execution.

<div align="center">

### 📞 Contact & Support

[![Telegram](https://img.shields.io/badge/Telegram-2CA5E0?style=for-the-badge&logo=telegram&logoColor=white)](https://t.me/heliusdevlabs)

**💬 Get in touch for support, questions, or collaboration**

</div>

## Features

### 🚀 **0-Slot Execution**
- **Jito Bundle Submission**: Submit transactions as bundles for atomic execution
- **Nozomi Confirmation Service**: Alternative confirmation service for fast execution
- **Priority Fee Optimization**: Dynamic gas price calculation based on network conditions
- **Bundle Timeout Management**: Configurable timeout and retry logic

### 📡 **Real-time Data Streaming**
- **gRPC Integration**: Stream new token listings and price updates
- **WebSocket Support**: Real-time market data feeds
- **Low Latency**: Sub-100ms data processing and execution
- **High Throughput**: Handle multiple concurrent opportunities

### 🎯 **MEV Strategies**
- **Arbitrage**: Cross-pool price differences
- **Front-running**: Execute before large trades
- **Back-running**: Execute after large trades
- **Sandwich Attacks**: Profit from price impact
- **Liquidation**: Liquidate undercollateralized positions

### 🛡️ **Risk Management**
- **Position Sizing**: Dynamic position sizing based on risk
- **Stop Loss**: Automatic stop loss protection
- **Take Profit**: Automatic profit taking
- **Daily Loss Limits**: Configurable daily loss limits
- **Slippage Protection**: Maximum slippage controls

### 📊 **Monitoring & Analytics**
- **Prometheus Metrics**: Comprehensive performance metrics
- **Real-time Logging**: Detailed trade and execution logs
- **Performance Tracking**: Latency, success rate, and profit tracking
- **Risk Monitoring**: Real-time risk metric monitoring

## Installation

### Prerequisites
- Rust 1.70+ 
- Solana CLI tools
- Valid Solana wallet with SOL for gas

### Build from Source

```bash
# Clone the repository
git clone <repository-url>
cd pumpswap-sniper-bot-0slot

# Build the project
cargo build --release

# Run the bot
cargo run --release -- --help
```

## Configuration

### Basic Configuration

Create a `config.toml` file:

```toml
# PumpSwap gRPC configuration
pumpswap_grpc_url = "https://grpc.pumpswap.fun:443"
pumpswap_api_key = "your_api_key_here"

# Solana RPC configuration
solana_rpc_url = "https://api.mainnet-beta.solana.com"
solana_ws_url = "wss://api.mainnet-beta.solana.com"

# Wallet configuration
private_key = "your_private_key_here"

# Sniper configuration
target_tokens = ["token_address_1", "token_address_2"]
min_liquidity = 10.0  # Minimum liquidity in SOL
max_slippage = 5.0    # Maximum slippage percentage
snipe_amount = 1.0    # Amount to snipe in SOL
max_gas_price = 1000000  # Maximum gas price in lamports

# MEV configuration
enable_mev = true
mev_strategies = ["arbitrage", "frontrun", "backrun"]
max_mev_profit = 1000.0

# Confirmation service
confirmation_service = "jito"  # "jito" or "nozomi"

# Jito configuration
jito_url = "https://mainnet.block-engine.jito.wtf"
jito_tip_account = "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY"
jito_tip_amount = 10000

# Nozomi configuration
nozomi_url = "https://api.nozomi.com"
nozomi_api_key = "your_nozomi_api_key"

# Performance settings
max_concurrent_trades = 5
transaction_timeout = 30
retry_attempts = 3
retry_delay = 1000

# Risk management
max_daily_loss = 100.0
max_position_size = 10.0
stop_loss_percentage = 10.0
take_profit_percentage = 50.0

# Monitoring
enable_metrics = true
metrics_port = 9090
log_level = "info"
```

### Advanced Configuration

```toml
# Advanced MEV settings
priority_fee_multiplier = 1.5
bundle_timeout = 5000
max_bundle_size = 10
enable_frontrunning_protection = true

# Performance optimization
max_concurrent_trades = 10
transaction_timeout = 15
retry_attempts = 5
retry_delay = 500

# Risk management
max_daily_loss = 50.0
max_position_size = 5.0
stop_loss_percentage = 5.0
take_profit_percentage = 25.0
```

## Usage

### Basic Usage

```bash
# Run with default configuration
cargo run --release

# Run with custom configuration
cargo run --release -- --config custom_config.toml

# Run with specific tokens
cargo run --release -- --tokens token1 token2 token3

# Run with custom settings
cargo run --release -- \
    --min-liquidity 20.0 \
    --max-slippage 3.0 \
    --snipe-amount 2.0 \
    --use-jito
```

### Advanced Usage

```bash
# Run with Nozomi confirmation service
cargo run --release -- --use-nozomi

# Run with debug logging
cargo run --release -- --debug

# Run with custom gas settings
cargo run --release -- \
    --max-gas-price 2000000 \
    --min-liquidity 50.0 \
    --max-slippage 2.0
```

### Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `--config` | Configuration file path | `config.toml` |
| `--debug` | Enable debug logging | `false` |
| `--tokens` | Target token addresses | `[]` |
| `--min-liquidity` | Minimum liquidity in SOL | `10.0` |
| `--max-slippage` | Maximum slippage percentage | `5.0` |
| `--snipe-amount` | Snipe amount in SOL | `1.0` |
| `--use-jito` | Use Jito confirmation service | `false` |
| `--use-nozomi` | Use Nozomi confirmation service | `false` |
| `--max-gas-price` | Maximum gas price in lamports | `1000000` |

## MEV Strategies

### Arbitrage
- **Description**: Profit from price differences between pools
- **Risk Level**: Low
- **Expected Profit**: 0.1-1.0 SOL
- **Execution Time**: 1-5 seconds

### Front-running
- **Description**: Execute trades before large transactions
- **Risk Level**: Medium
- **Expected Profit**: 0.5-5.0 SOL
- **Execution Time**: <1 second

### Back-running
- **Description**: Execute trades after large transactions
- **Risk Level**: Low
- **Expected Profit**: 0.2-2.0 SOL
- **Execution Time**: 1-3 seconds

### Sandwich Attacks
- **Description**: Profit from price impact of large trades
- **Risk Level**: High
- **Expected Profit**: 1.0-10.0 SOL
- **Execution Time**: <1 second

### Liquidation
- **Description**: Liquidate undercollateralized positions
- **Risk Level**: Medium
- **Expected Profit**: 0.5-5.0 SOL
- **Execution Time**: 2-10 seconds

## Monitoring

### Prometheus Metrics

The bot exposes metrics on port 9090 by default:

- `trades_executed_total`: Total number of trades executed
- `trades_successful_total`: Total number of successful trades
- `trades_failed_total`: Total number of failed trades
- `mev_opportunities_total`: Total number of MEV opportunities detected
- `mev_executed_total`: Total number of MEV strategies executed
- `profit_earned_sol`: Total profit earned in SOL
- `gas_spent_lamports`: Total gas spent in lamports
- `active_trades`: Number of currently active trades
- `execution_latency_ms`: Trade execution latency in milliseconds
- `bundle_confirmation_time_ms`: Bundle confirmation time in milliseconds

### Grafana Dashboard

Import the provided Grafana dashboard for comprehensive monitoring:

```bash
# Access Grafana at http://localhost:3000
# Import dashboard from grafana-dashboard.json
```

## Performance Optimization

### Network Optimization
- Use dedicated RPC endpoints
- Enable connection pooling
- Optimize WebSocket connections
- Use CDN for static resources

### Execution Optimization
- Pre-compute transaction templates
- Use parallel execution where possible
- Optimize gas price calculations
- Implement transaction batching

### Memory Optimization
- Use object pooling for transactions
- Implement efficient data structures
- Monitor memory usage
- Garbage collection tuning

## Risk Management

### Position Sizing
- Dynamic position sizing based on account balance
- Risk percentage per trade
- Maximum position size limits
- Correlation analysis

### Stop Loss
- Automatic stop loss triggers
- Trailing stop loss
- Time-based stop loss
- Volatility-based stop loss

### Take Profit
- Automatic profit taking
- Partial profit taking
- Scaling out strategies
- Profit target optimization

## Troubleshooting

### Common Issues

1. **High Gas Prices**
   - Reduce `max_gas_price` in configuration
   - Use lower priority fee multiplier
   - Optimize transaction size

2. **Failed Transactions**
   - Check account balance
   - Verify token addresses
   - Review slippage settings
   - Check network congestion

3. **Low Profitability**
   - Increase `min_liquidity` threshold
   - Optimize MEV strategies
   - Reduce transaction costs
   - Improve execution timing

4. **Connection Issues**
   - Check RPC endpoint availability
   - Verify network connectivity
   - Review firewall settings
   - Test with different endpoints

### Debug Mode

Enable debug logging for detailed information:

```bash
cargo run --release -- --debug
```

### Performance Monitoring

Monitor key metrics:
- Execution latency
- Success rate
- Profit per trade
- Gas efficiency
- Network utilization

## Security

### Wallet Security
- Use hardware wallets for production
- Implement multi-signature wallets
- Regular key rotation
- Secure key storage

### Network Security
- Use HTTPS for all connections
- Implement rate limiting
- Monitor for suspicious activity
- Regular security audits

### Code Security
- Regular dependency updates
- Code review process
- Security testing
- Vulnerability scanning

## Legal and Compliance

### Regulatory Compliance
- Understand local regulations
- Implement compliance measures
- Regular legal reviews
- Documentation requirements

### Terms of Service
- Review platform terms
- Understand usage limits
- Monitor for changes
- Compliance monitoring

## Support

### Documentation
- API documentation
- Configuration reference
- Troubleshooting guide
- Performance tuning guide

### Community
- Discord server
- Telegram group
- GitHub issues
- Stack Overflow

### Professional Support
- Enterprise support
- Custom development
- Training and consulting
- SLA agreements

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Disclaimer

This software is for educational and research purposes only. Users are responsible for ensuring compliance with all applicable laws and regulations. The authors are not responsible for any financial losses or legal issues arising from the use of this software.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## Changelog

### v0.1.0
- Initial release
- Basic sniper functionality
- Jito and Nozomi integration
- MEV strategy support
- Monitoring and metrics
