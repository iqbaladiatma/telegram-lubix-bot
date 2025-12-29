use serde::Deserialize;
use std::collections::HashMap;

// ==========================================
// API RESPONSE STRUCTURES
// ==========================================

#[derive(Deserialize, Debug, Clone)]
pub struct SyariahApiResponse {
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub sector: Option<String>,
    #[serde(default)]
    pub industry: Option<String>,
    #[serde(rename = "marketCap", default)]
    pub market_cap: Option<f64>,
    #[serde(default)]
    pub shares: Option<f64>,
    #[serde(default)]
    pub issi: Option<bool>,
    #[serde(rename = "syariahIndicator", default)]
    pub syariah_indicator: Option<SyariahIndicatorApi>,
    #[serde(default)]
    pub harga: Option<StockPriceApi>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct SyariahIndicatorApi {
    #[serde(rename = "hutangBunga", default)]
    pub hutang_bunga: Option<serde_json::Value>,
    #[serde(rename = "nonHalal", default)]
    pub non_halal: Option<serde_json::Value>,
    #[serde(default)]
    pub business: Option<bool>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct StockPriceApi {
    #[serde(default)]
    pub now: Option<i64>,
    #[serde(rename = "deltaPrice", default)]
    pub delta_price: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct CmcApiResponse {
    pub data: HashMap<String, CmcCryptoData>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CmcCryptoData {
    pub name: String,
    pub symbol: String,
    pub quote: HashMap<String, CmcQuoteData>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CmcQuoteData {
    pub price: f64,
    #[serde(rename = "percent_change_1h", default)]
    pub change_1h: f64,
    #[serde(rename = "percent_change_24h", default)]
    pub change_24h: f64,
    #[serde(rename = "percent_change_7d", default)]
    pub change_7d: f64,
    #[serde(rename = "market_cap", default)]
    pub market_cap: f64,
    #[serde(rename = "volume_24h", default)]
    pub volume_24h: f64,
}

#[derive(Deserialize, Debug)]
pub struct DexScreenerResponse {
    pub pairs: Option<Vec<DexPairData>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DexPairData {
    #[serde(rename = "baseToken")]
    pub base_token: DexTokenData,
    #[serde(rename = "priceUsd", default)]
    pub price_usd: Option<String>,
    #[serde(rename = "priceChange", default)]
    pub price_change: Option<DexPriceChange>,
    #[serde(default)]
    pub liquidity: Option<DexLiquidity>,
    #[serde(default)]
    pub volume: Option<DexVolume>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct DexTokenData {
    pub name: String,
    pub symbol: String,
    pub address: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct DexPriceChange {
    #[serde(default)]
    pub h1: Option<f64>,
    #[serde(default)]
    pub h24: Option<f64>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct DexLiquidity {
    #[serde(default)]
    pub usd: Option<f64>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct DexVolume {
    #[serde(default)]
    pub h24: Option<f64>,
}

#[derive(Deserialize, Debug)]
pub struct FearGreedResponse {
    pub data: Vec<FearGreedData>,
}

#[derive(Deserialize, Debug)]
pub struct FearGreedData {
    pub value: String,
    pub value_classification: String,
}

// ==========================================
// HELPER FUNCTIONS
// ==========================================

pub fn format_number(n: f64) -> String {
    let s = format!("{:.0}", n);
    let mut res = String::new();
    let len = s.len();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            res.push('.');
        }
        res.push(c);
    }
    res
}

// ==========================================
// API FUNCTIONS
// ==========================================

pub async fn fetch_stock_from_api(ticker: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let ticker_upper = ticker.to_uppercase();
    
    let url = format!("https://syariahsaham-api.fly.dev/emiten/{}", ticker_upper);
    
    let response = client.get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Saham {} tidak ditemukan di ISSI", ticker_upper));
    }
    
    let stock: SyariahApiResponse = response.json().await
        .map_err(|_| format!("Saham {} tidak ditemukan", ticker_upper))?;
    
    let price = stock.harga.as_ref().and_then(|h| h.now).unwrap_or(0);
    let change = stock.harga.as_ref().and_then(|h| h.delta_price).unwrap_or(0);
    let change_emoji = if change >= 0 { "📈" } else { "📉" };
    let change_sign = if change >= 0 { "+" } else { "" };
    
    let sector = stock.sector.unwrap_or_else(|| "N/A".to_string());
    let market_cap = stock.market_cap.unwrap_or(0.0);
    let is_issi = stock.issi.unwrap_or(false);
    
    let hutang = stock.syariah_indicator.as_ref()
        .and_then(|s| s.hutang_bunga.as_ref())
        .map(|v| match v {
            serde_json::Value::Number(n) => format!("{:.2}%", n.as_f64().unwrap_or(0.0)),
            _ => "N/A".to_string()
        })
        .unwrap_or_else(|| "N/A".to_string());
    
    let non_halal = stock.syariah_indicator.as_ref()
        .and_then(|s| s.non_halal.as_ref())
        .map(|v| match v {
            serde_json::Value::Number(n) => format!("{:.2}%", n.as_f64().unwrap_or(0.0)),
            _ => "N/A".to_string()
        })
        .unwrap_or_else(|| "N/A".to_string());

    Ok(format!(
        "🕌 <b>{} - {}</b>\n========================\n💰 <b>HARGA SAHAM:</b>\n• Current: <code>Rp {}</code>\n• Change: {} <code>{}{}</code>\n\n📊 <b>FUNDAMENTAL:</b>\n• Sektor: <code>{}</code>\n• Market Cap: <code>Rp {}T</code>\n• Status: {} <code>{}</code>\n\n🕌 <b>SYARIAH STATUS:</b>\n• ISSI Listed: <code>{}</code>\n• Hutang Bunga: <code>{}</code>\n• Non-Halal: <code>{}</code>\n\n========================\n<i>💡 Data real-time dari Syariah API</i>",
        stock.code, stock.name,
        format_number(price as f64),
        change_emoji, change_sign, change,
        sector,
        format!("{:.2}", market_cap / 1_000_000_000_000.0),
        if is_issi { "✅" } else { "❌" },
        if is_issi { "HALAL" } else { "NON-HALAL" },
        if is_issi { "✅ YA" } else { "❌ TIDAK" },
        hutang, non_halal
    ))
}

pub async fn fetch_crypto_from_cmc(symbol: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let api_key = std::env::var("CMC_API_KEY").unwrap_or_default();
    let symbol_upper = symbol.to_uppercase();
    
    let url = format!(
        "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?symbol={}&convert=USD",
        symbol_upper
    );
    
    let response = client.get(&url)
        .header("X-CMC_PRO_API_KEY", &api_key)
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Crypto {} tidak ditemukan", symbol_upper));
    }
    
    let data: CmcApiResponse = response.json().await
        .map_err(|_| format!("Crypto {} tidak ditemukan", symbol_upper))?;
    
    let crypto = data.data.get(&symbol_upper)
        .ok_or_else(|| format!("Crypto {} tidak ditemukan", symbol_upper))?;
    
    let quote = crypto.quote.get("USD")
        .ok_or_else(|| "No USD quote".to_string())?;
    
    let idr_price = quote.price * 15800.0;
    
    let ch1 = if quote.change_1h >= 0.0 { "📈" } else { "📉" };
    let ch24 = if quote.change_24h >= 0.0 { "📈" } else { "📉" };
    let ch7d = if quote.change_7d >= 0.0 { "📈" } else { "📉" };
    
    Ok(format!(
        "🪙 <b>{} - {}</b>\n========================\n💰 <b>PRICE DATA:</b>\n• USD: <code>${:.4}</code>\n• IDR: <code>Rp {}</code>\n\n📊 <b>PRICE CHANGES:</b>\n• 1H: {} <code>{:+.2}%</code>\n• 24H: {} <code>{:+.2}%</code>\n• 7D: {} <code>{:+.2}%</code>\n\n📈 <b>MARKET DATA:</b>\n• Market Cap: <code>${}</code>\n• Volume 24H: <code>${}</code>\n\n========================\n<i>💡 Data real-time dari CoinMarketCap</i>",
        crypto.symbol, crypto.name,
        quote.price, format_number(idr_price),
        ch1, quote.change_1h,
        ch24, quote.change_24h,
        ch7d, quote.change_7d,
        format_number(quote.market_cap),
        format_number(quote.volume_24h)
    ))
}

pub async fn fetch_solana_token(query: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    
    let url = if query.len() > 30 {
        format!("https://api.dexscreener.com/latest/dex/tokens/{}", query)
    } else {
        format!("https://api.dexscreener.com/latest/dex/search?q={}", query)
    };
    
    let response = client.get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;
    
    let data: DexScreenerResponse = response.json().await
        .map_err(|_| format!("Token {} tidak ditemukan", query))?;
    
    let pair = data.pairs
        .and_then(|p| p.into_iter().find(|x| x.base_token.symbol.to_uppercase().contains(&query.to_uppercase()) || x.base_token.address.contains(query)))
        .ok_or_else(|| format!("Token {} tidak ditemukan", query))?;
    
    let price = pair.price_usd.unwrap_or_else(|| "0".to_string());
    let change_1h = pair.price_change.as_ref().and_then(|p| p.h1).unwrap_or(0.0);
    let change_24h = pair.price_change.as_ref().and_then(|p| p.h24).unwrap_or(0.0);
    let liquidity = pair.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0);
    let volume = pair.volume.as_ref().and_then(|v| v.h24).unwrap_or(0.0);
    
    let ch1 = if change_1h >= 0.0 { "📈" } else { "📉" };
    let ch24 = if change_24h >= 0.0 { "📈" } else { "📉" };
    
    Ok(format!(
        "⚡️ <b>{} - {}</b>\n========================\n💰 <b>TOKEN PRICE:</b>\n• Price: <code>${}</code>\n• 1H: {} <code>{:+.2}%</code>\n• 24H: {} <code>{:+.2}%</code>\n\n🔗 <b>CONTRACT INFO:</b>\n• CA: <code>{}</code>\n• Network: <code>Solana</code>\n\n📊 <b>MARKET DATA:</b>\n• Liquidity: <code>${}</code>\n• Volume 24H: <code>${}</code>\n\n========================\n<i>💡 Data from DexScreener</i>",
        pair.base_token.symbol, pair.base_token.name,
        price, ch1, change_1h, ch24, change_24h,
        pair.base_token.address,
        format_number(liquidity), format_number(volume)
    ))
}

pub async fn fetch_fear_greed_index() -> Result<String, String> {
    let client = reqwest::Client::new();
    
    let response = client.get("https://api.alternative.me/fng/")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;
    
    let data: FearGreedResponse = response.json().await
        .map_err(|_| "Failed to parse Fear & Greed data".to_string())?;
    
    let fng = data.data.first().ok_or("No data available")?;
    let value: i32 = fng.value.parse().unwrap_or(0);
    
    let emoji = match value {
        0..=25 => "😱",
        26..=45 => "😰",
        46..=55 => "😐",
        56..=75 => "😊",
        _ => "🤑",
    };
    
    let bar = "█".repeat((value / 5) as usize) + &"░".repeat((20 - value / 5) as usize);
    
    Ok(format!(
        "🌡 <b>FEAR & GREED INDEX</b>\n========================\n\n{} <b>Score: {}/100</b>\n<code>[{}]</code>\n\n📊 <b>Classification:</b> <code>{}</code>\n\n<i>💡 0 = Extreme Fear, 100 = Extreme Greed</i>\n========================\n<i>Source: Alternative.me</i>",
        emoji, value, bar, fng.value_classification
    ))
}

pub async fn fetch_market_pulse() -> Result<String, String> {
    let client = reqwest::Client::new();
    let api_key = std::env::var("CMC_API_KEY").unwrap_or_default();
    
    let url = "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?symbol=BTC,ETH,SOL,BNB,XRP&convert=USD";
    
    let response = client.get(url)
        .header("X-CMC_PRO_API_KEY", &api_key)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;
    
    let data: CmcApiResponse = response.json().await
        .map_err(|_| "Failed to fetch market data".to_string())?;
    
    let mut result = "📈 <b>MARKET PULSE</b>\n========================\n\n".to_string();
    
    for symbol in &["BTC", "ETH", "SOL", "BNB", "XRP"] {
        if let Some(crypto) = data.data.get(*symbol) {
            if let Some(quote) = crypto.quote.get("USD") {
                let ch = if quote.change_24h >= 0.0 { "📈" } else { "📉" };
                result.push_str(&format!(
                    "{} <b>{}</b>: <code>${:.2}</code> {} <code>{:+.2}%</code>\n",
                    ch, symbol, quote.price, ch, quote.change_24h
                ));
            }
        }
    }
    
    result.push_str("\n========================\n<i>💡 24H Changes - CoinMarketCap</i>");
    Ok(result)
}

pub async fn get_real_crypto_price(symbol: &str) -> Result<f64, String> {
    let client = reqwest::Client::new();
    let api_key = std::env::var("CMC_API_KEY").unwrap_or_default();
    let symbol_upper = symbol.to_uppercase();
    
    let url = format!(
        "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?symbol={}&convert=USD",
        symbol_upper
    );
    
    let response = client.get(&url)
        .header("X-CMC_PRO_API_KEY", &api_key)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Error: {}", e))?;
    
    let data: CmcApiResponse = response.json().await
        .map_err(|_| format!("Crypto {} tidak ditemukan", symbol_upper))?;
    
    let crypto = data.data.get(&symbol_upper)
        .ok_or_else(|| format!("Crypto {} tidak ditemukan", symbol_upper))?;
    
    let quote = crypto.quote.get("USD").ok_or("No quote")?;
    Ok(quote.price)
}
