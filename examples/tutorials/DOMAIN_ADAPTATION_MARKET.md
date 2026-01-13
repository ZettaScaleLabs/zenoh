# Domain Adaptation Guide: Market Data Distribution

This guide shows how to adapt the Smart Building tutorial patterns to real-time financial market data distribution, where prices, quotes, and trades flow at high speed to multiple trading systems and analytics platforms.

## Overview

Market data distribution shares core patterns with Smart Building: hierarchical organization, multiple subscribers at different scales, request/response for detailed information. However, it introduces challenges around:

- **Extremely high throughput** (thousands of messages per second)
- **Latency sensitivity** (microseconds matter)
- **Data validation** (must handle feeds down gracefully)
- **Time-series alignment** (accurate sequencing crucial)

The Zenoh patterns handle these naturally: hierarchical key expressions organize instruments, pub/sub scales efficiently, queries retrieve historical data or detailed quotes.

## Quick Mapping

| Smart Building Concept | Market Data Application |
|---|---|
| Building with zones | Market exchanges or exchanges with different asset classes |
| Room sensors | Price feeds for individual instruments |
| Thermostat control | Trade execution endpoints |
| Monitoring display | Trading dashboard |
| Multi-reader queryable | Historical quote lookup |
| Router for scale | Market data gateway for multiple traders |
| Persistence | Trade history and audit log |

## Architecture Pattern

```
Market Data Feed (Zenoh Publisher)
├─ Exchange NASDAQ
│  ├─ AAPL (price, bid/ask, volume)
│  ├─ MSFT (price, bid/ask, volume)
│  └─ GOOGL (price, bid/ask, volume)
├─ Exchange NYSE
│  ├─ IBM (price, bid/ask, volume)
│  ├─ JNJ (price, bid/ask, volume)
│  └─ JPM (price, bid/ask, volume)
├─ Bonds
│  ├─ US10Y (yield, price, spread)
│  └─ US2Y (yield, price, spread)
├─ Trading System (Subscriber: selective instruments, Querier: historical)
├─ Risk Dashboard (Subscriber: filtered view)
└─ Execution System (Queryable: order status)
```

## Key Expression Hierarchy

```
# By exchange and symbol
market/nasdaq/aapl/quote          # Latest price/bid/ask
market/nasdaq/aapl/trades         # Trade ticks
market/nasdaq/aapl/bid            # Best bid
market/nasdaq/aapl/ask            # Best ask
market/nyse/ibm/quote

# Alternative: By asset class
market/equities/nasdaq/aapl/quote
market/equities/nyse/ibm/quote
market/fixed-income/bonds/us10y/quote
market/derivatives/options/spy-call-420/quote

# Alternative: By industry
market/tech/nasdaq/aapl/quote
market/tech/nasdaq/msft/quote
market/finance/nyse/jpm/quote
market/healthcare/nyse/jnj/quote
```

## Chapter Adaptation Examples

### Chapter 1: Hello Zenoh → Single Instrument Feed

**Original:** Room temperature sensor
**Adapted:** Single stock price feed

```rust
use zenoh::prelude::*;
use std::time::Duration;
use rand::Rng;

#[tokio::main]
async fn main() {
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    // Publish stock price
    let publisher = session
        .declare_publisher("market/nasdaq/aapl/quote")
        .res()
        .await
        .unwrap();
    
    let mut price = 175.0;
    let mut rng = rand::thread_rng();
    
    loop {
        // Simulate price movement (random walk)
        price += rng.gen_range(-0.5..0.5);
        price = price.max(100.0).min(200.0);
        
        let bid = price - 0.01;
        let ask = price + 0.01;
        let volume = rng.gen_range(1000000..10000000);
        
        let quote = format!(
            r#"{{"symbol":"AAPL","price":{},"bid":{},"ask":{},"volume":{}}}"#,
            price, bid, ask, volume
        );
        
        publisher.put(quote).res().await.ok();
        println!("AAPL: ${:.2}", price);
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
```

### Chapter 2: Key Expressions → Market Instruments

**Key insight:** Design hierarchies for efficient filtering

```rust
// Subscribe to all NASDAQ stocks
let nasdaq_sub = session
    .declare_subscriber("market/nasdaq/*/quote")
    .res()
    .await
    .unwrap();

// Subscribe to technology sector stocks
let tech_sub = session
    .declare_subscriber("market/tech/*/*/quote")
    .res()
    .await
    .unwrap();

// Subscribe to bid/ask spreads (for market makers)
let spread_sub = session
    .declare_subscriber("market/*/*/bid")
    .res()
    .await
    .unwrap();

while let Ok(sample) = nasdaq_sub.recv_async().await {
    let key = sample.key_expr().as_str();
    let parts: Vec<&str> = key.split('/').collect();
    let exchange = parts[1];
    let symbol = parts[2];
    
    println!("Received quote for {} on {}", symbol, exchange);
}
```

### Chapter 3: Query/Reply → Historical Quote Lookup

**Original:** Thermostat responds with status
**Adapted:** Quote engine responds with detailed quote or trade history

```rust
use zenoh::prelude::*;
use serde_json::json;

#[tokio::main]
async fn main() {
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    // Publisher: stream current quotes
    let quote_pub = session
        .declare_publisher("market/nasdaq/aapl/quote")
        .res()
        .await
        .unwrap();
    
    // Queryable: respond to detailed quote requests
    let mut queries = session
        .declare_queryable("market/nasdaq/aapl/details")
        .res()
        .await
        .unwrap();
    
    // Spawn publisher
    tokio::spawn(async move {
        let mut price = 175.0;
        let mut rng = rand::thread_rng();
        
        for _ in 0..1000 {
            price += rng.gen_range(-0.5..0.5);
            let quote = format!(r#"{{"price":{}}}"#, price);
            let _ = quote_pub.put(quote).res().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });
    
    // Handle detail queries
    while let Ok(query) = queries.recv_async().await {
        let details = json!({
            "symbol": "AAPL",
            "exchange": "NASDAQ",
            "sector": "Technology",
            "market_cap_b": 2800.0,
            "pe_ratio": 28.5,
            "dividend_yield": 0.4,
            "52w_high": 199.62,
            "52w_low": 124.17,
            "avg_volume_m": 52.3,
            "last_update": chrono::Utc::now().to_rfc3339()
        });
        
        let _ = query.reply(Ok(Sample::new(
            query.key_expr().clone(),
            serde_json::to_string(&details).unwrap()
        ))).res().await;
    }
}
```

### Chapter 4: Router Setup → Multi-Exchange Feed Gateway

**Original:** Router connects multiple buildings
**Adapted:** Router distributes market data from multiple exchanges

```bash
# Market data gateway configuration (market_gateway.json5)
router: {
    storages: [
        {
            key_prefix: "market/nasdaq/**",
            volume: "memory",
            capacity: 50000,  # Last 50k NASDAQ quotes
            keep_last: 1,     # Keep only latest quote per instrument
        },
        {
            key_prefix: "market/nyse/**",
            volume: "memory",
            capacity: 50000,
            keep_last: 1,
        },
        {
            key_prefix: "trades/executed/**",
            volume: "filesystem",
            dir: "/data/trades",
            keep_last: 10000,  # Keep last 10k executed trades for audit
        }
    ]
}

# Run market data gateway
zenohd -c market_gateway.json5 --listen 0.0.0.0:7447
```

Multiple feeds can connect:
```bash
# NASDAQ price feed publishes to gateway
cargo run --bin nasdaq_feed -- --gateway 127.0.0.1:7447

# NYSE price feed publishes to gateway
cargo run --bin nyse_feed -- --gateway 127.0.0.1:7447

# Trading system subscribes from gateway
cargo run --bin trading_system -- --gateway 127.0.0.1:7447

# Risk dashboard queries from gateway
cargo run --bin risk_dashboard -- --gateway 127.0.0.1:7447
```

### Chapter 5: Multi-Tier System → Trading Infrastructure

**Original:** Multi-reader storage with aggregation
**Adapted:** Market data aggregation, risk monitoring, execution

**Architecture:**
- **Layer 1 (Feeds):** Exchange feeds publish raw prices
- **Layer 2 (Aggregation):** Gateway consolidates multiple feeds, deduplication
- **Layer 3 (Risk):** Risk system monitors positions and market exposure
- **Layer 4 (Execution):** Trading system executes based on prices and risk

```rust
// Market aggregator: deduplicate and normalize prices
#[tokio::main]
async fn main() {
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    // Subscribe to all instrument prices from all exchanges
    let mut subscriber = session
        .declare_subscriber("market/*/*/quote")
        .res()
        .await
        .unwrap();
    
    // Publish normalized/deduplicated quotes
    let normalized_pub = session
        .declare_publisher("market/consolidated/*/quote")
        .res()
        .await
        .unwrap();
    
    // Track last price by instrument to avoid duplicates
    use std::collections::HashMap;
    let mut last_prices: HashMap<String, f64> = HashMap::new();
    
    while let Ok(sample) = subscriber.recv_async().await {
        let key = sample.key_expr().as_str();
        let payload = sample.value().to_string();
        
        // Parse and check if new
        if let Ok(quote) = serde_json::from_str::<serde_json::Value>(&payload) {
            let price = quote["price"].as_f64().unwrap_or(0.0);
            let symbol = quote["symbol"].as_str().unwrap_or("");
            
            // Only publish if price changed (not duplicate from multiple feeds)
            if last_prices.get(symbol).copied() != Some(price) {
                last_prices.insert(symbol.to_string(), price);
                let _ = normalized_pub.put(payload).res().await;
            }
        }
    }
}

// Risk monitor: track portfolio exposure
#[tokio::main]
async fn main() {
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    // Subscribe to all instrument prices
    let mut price_sub = session
        .declare_subscriber("market/*/*/quote")
        .res()
        .await
        .unwrap();
    
    // Alert publisher for risk events
    let risk_pub = session
        .declare_publisher("risk-events/alerts")
        .res()
        .await
        .unwrap();
    
    // Portfolio: positions and stop-loss levels
    let mut portfolio = get_portfolio_positions();
    
    while let Ok(sample) = price_sub.recv_async().await {
        let symbol = extract_symbol(sample.key_expr());
        if let Ok(quote) = serde_json::from_str::<serde_json::Value>(&sample.value().to_string()) {
            let price = quote["price"].as_f64().unwrap_or(0.0);
            
            if let Some(position) = portfolio.get_mut(symbol) {
                let pnl = (price - position.entry_price) * position.quantity as f64;
                
                // Alert on significant losses
                if pnl < position.stop_loss_limit {
                    let alert = json!({
                        "symbol": symbol,
                        "price": price,
                        "pnl": pnl,
                        "action": "consider_exit"
                    });
                    let _ = risk_pub.put(serde_json::to_string(&alert).unwrap()).res().await;
                }
            }
        }
    }
}

// Trading system: execute based on prices
#[tokio::main]
async fn main() {
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    // Subscribe to prices of interest
    let mut subscriber = session
        .declare_subscriber("market/nasdaq/*/quote")  // NASDAQ only
        .res()
        .await
        .unwrap();
    
    // Execution endpoint (queryable)
    let mut executions = session
        .declare_queryable("execution/place-order")
        .res()
        .await
        .unwrap();
    
    // Track watching symbols
    let watch_list = vec!["AAPL", "MSFT", "GOOGL", "AMZN", "NVDA"];
    let mut prices = HashMap::new();
    
    while let Ok(sample) = subscriber.recv_async().await {
        let symbol = extract_symbol(sample.key_expr());
        if watch_list.contains(&symbol.as_str()) {
            if let Ok(quote) = serde_json::from_str::<serde_json::Value>(&sample.value().to_string()) {
                let price = quote["price"].as_f64().unwrap_or(0.0);
                prices.insert(symbol.clone(), price);
                
                // Simple strategy: buy if price < target, sell if > target
                if let Some(target) = get_target_price(&symbol) {
                    if price < target * 0.98 {
                        execute_trade("BUY", &symbol, 100, price);
                    } else if price > target * 1.02 {
                        execute_trade("SELL", &symbol, 100, price);
                    }
                }
            }
        }
    }
}
```

## Complete Example: Multi-Exchange Price Feed

```rust
use zenoh::prelude::*;
use std::time::Duration;
use std::collections::HashMap;
use serde_json::json;

#[derive(Clone)]
struct Instrument {
    symbol: String,
    exchange: String,
    price: f64,
    bid: f64,
    ask: f64,
}

#[tokio::main]
async fn main() {
    let exchange = std::env::args()
        .nth(1)
        .map(|s| s.to_uppercase())
        .unwrap_or_else(|| "NASDAQ".to_string());
    
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    // Define instruments by exchange
    let instruments = match exchange.as_str() {
        "NASDAQ" => vec![
            Instrument { symbol: "AAPL".into(), exchange: exchange.clone(), price: 175.0, bid: 174.99, ask: 175.01 },
            Instrument { symbol: "MSFT".into(), exchange: exchange.clone(), price: 330.0, bid: 329.99, ask: 330.01 },
            Instrument { symbol: "GOOGL".into(), exchange: exchange.clone(), price: 140.0, bid: 139.99, ask: 140.01 },
        ],
        "NYSE" => vec![
            Instrument { symbol: "IBM".into(), exchange: exchange.clone(), price: 165.0, bid: 164.99, ask: 165.01 },
            Instrument { symbol: "JNJ".into(), exchange: exchange.clone(), price: 160.0, bid: 159.99, ask: 160.01 },
        ],
        _ => vec![],
    };
    
    let mut rng = rand::thread_rng();
    let mut current_prices: HashMap<String, Instrument> = instruments
        .iter()
        .map(|i| (i.symbol.clone(), i.clone()))
        .collect();
    
    // Publishers for each instrument
    let mut publishers = HashMap::new();
    for instrument in instruments {
        let pub_key = format!("market/{}/{}/quote", 
            exchange.to_lowercase(), 
            instrument.symbol.to_lowercase());
        let publisher = session
            .declare_publisher(&pub_key)
            .res()
            .await
            .unwrap();
        publishers.insert(instrument.symbol.clone(), publisher);
    }
    
    // Publishing loop
    loop {
        for (symbol, publisher) in &publishers {
            if let Some(instrument) = current_prices.get_mut(symbol) {
                // Simulate price movement
                instrument.price += rng.gen_range(-0.2..0.2);
                instrument.bid = instrument.price - 0.01;
                instrument.ask = instrument.price + 0.01;
                
                let quote = json!({
                    "symbol": symbol,
                    "exchange": &exchange,
                    "price": instrument.price,
                    "bid": instrument.bid,
                    "ask": instrument.ask,
                    "timestamp": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                });
                
                let _ = publisher.put(serde_json::to_string(&quote).unwrap()).res().await;
            }
        }
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
```

## Exercises

### Exercise 1: Multi-Exchange Feed
Run feeds for both NASDAQ and NYSE, then subscribe to all prices. Verify quotes arrive from both exchanges.

```bash
# Terminal 1-2: Feeds
cargo run -- nasdaq
cargo run -- nyse

# Terminal 3: All quotes
cargo run --example z_sub -- --selector 'market/*/*/quote'
```

### Exercise 2: Bid/Ask Spread Monitor
Subscribe to bid/ask prices and calculate spread as percentage. Alert if spread widens abnormally.

**Key concepts:**
- Correlate bid and ask streams
- Calculate spread percentage
- Threshold-based alerting

### Exercise 3: Portfolio Performance Dashboard
Query current prices, calculate portfolio P&L, and display top gainers/losers.

**Key concepts:**
- Historical position tracking
- Price aggregation
- Performance metrics calculation

## Common Patterns

### Selective Subscription (Watchlist)
```rust
// Only subscribe to specific symbols
let watchlist = vec!["AAPL", "MSFT", "GOOGL"];
for symbol in watchlist {
    session.declare_subscriber(format!("market/nasdaq/{}/quote", symbol.to_lowercase()))
        .res()
        .await?;
}
```

### High-Frequency Data Handling
```rust
// Batch updates for efficiency
let mut batch = Vec::new();
while let Ok(sample) = subscriber.recv_async().await {
    batch.push(sample);
    if batch.len() >= 100 {
        process_batch(&batch);
        batch.clear();
    }
}

// Or use selective subscriptions to reduce traffic
session.declare_subscriber("market/*/*/bid")  // Only bid side
```

### Price History Query
```rust
// Query last N prices for an instrument
let selector = "market/nasdaq/aapl/quote";
let queryable = session.declare_queryable(selector).res().await?;

while let Ok(query) = queryable.recv_async().await {
    let history = get_price_history("AAPL", 100);  // Last 100 ticks
    query.reply(Ok(Sample::new(
        query.key_expr().clone(),
        serde_json::to_string(&history)?
    ))).res().await?;
}
```

## Integration with Trading Systems

The market data pattern integrates with existing trading infrastructure:

```rust
// Send prices to analytics engine
// Order execution through queries
// Risk monitoring via subscriptions
// Audit logging via storage

// Example: FIX protocol bridge
while let Ok(sample) = subscriber.recv_async().await {
    let fix_message = convert_to_fix(&sample);
    send_fix_message(fix_socket, &fix_message).await?;
}
```

## Next Steps

1. Start with **Chapter 1: Hello Zenoh** using single instrument feed
2. Modify **Chapter 2: Key Expressions** to organize by exchange/sector
3. Extend **Chapter 3: Query/Reply** for quote lookup
4. Build **Chapter 4: Router Setup** for multi-exchange gateway
5. Design **Chapter 5: Multi-Tier** with risk monitoring and execution

## References

- [Smart Building Tutorial](./smart_building/README.md)
- [Chapter 2: Key Expressions](./smart_building/chapter_2/README.md)
- [Chapter 4: Router Setup](./smart_building/chapter_4/README.md)
- [Zenoh Pub/Sub Performance](https://zenoh.io/docs/manual/pubsub/)
