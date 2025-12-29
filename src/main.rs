use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode, BotCommand};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::env;

// --- SOLANA LIBS ---
use solana_sdk::{signature::{Keypair, Signer}};
use solana_client::rpc_client::RpcClient;
use base64::{Engine as _, engine::general_purpose};

// ==========================================
// 1. STRUKTUR DATA & STATE
// ==========================================

#[derive(Clone, PartialEq)]
enum UserState { 
    Idle, 
    AwaitingCrypto, AwaitingStock, AwaitingSentiment, 
    AwaitingSolanaTicker,
    AwaitingBuyTicker, AwaitingSellTicker, AwaitingAddWatchlist,
    AwaitingRealBuyCA,
    AwaitingBroadcast, AwaitingBanUser, AwaitingUnbanUser, 
    AwaitingGiftSaldo, AwaitingDirectMsg
}

#[derive(Debug, Clone)]
struct Holding { symbol: String, quantity: f64, avg_price: f64 }
#[derive(Debug, Clone)]
struct UserPortfolio { balance: f64, holdings: HashMap<String, Holding> }

// --- DATA MODELS ---
#[derive(Deserialize, Debug, Clone)]
struct SyariahIndicator { #[serde(rename = "hutangBunga")] hutang_bunga: serde_json::Value, #[serde(rename = "nonHalal")] non_halal: serde_json::Value, business: bool }
#[derive(Deserialize, Debug, Clone)]
struct StockPrice { now: i64, #[serde(rename = "deltaPrice")] change: i64 }
#[derive(Deserialize, Debug, Clone)]
struct EmitenDetail { 
    #[serde(rename = "code")] kode: String, #[serde(rename = "name")] nama: String, 
    sector: String, industry: String, ipo: String, papan: String, index: String, 
    #[serde(rename = "marketCap")] market_cap: f64, shares: f64, issi: bool, 
    #[serde(rename = "syariahIndicator")] syariah_indicator: SyariahIndicator, harga: StockPrice 
}

#[derive(Deserialize, Debug, Clone)]
struct CmcResponse { data: HashMap<String, CryptoData> }
#[derive(Deserialize, Debug, Clone)]
struct CryptoData { name: String, symbol: String, quote: HashMap<String, QuoteData> }
#[derive(Deserialize, Debug, Clone)]
struct QuoteData { 
    price: f64,
    #[serde(rename = "percent_change_1h")] change_1h: f64,
    #[serde(rename = "percent_change_24h")] change_24h: f64,
    #[serde(rename = "percent_change_7d")] change_7d: f64,
    #[serde(rename = "market_cap")] market_cap: f64,
    #[serde(rename = "volume_24h")] volume_24h: f64
}

#[derive(Deserialize, Debug, Clone)]
struct AltTickerResponse { data: Option<HashMap<String, AltTickerData>> }
#[derive(Deserialize, Debug, Clone)]
struct AltTickerData { name: String, symbol: String, quotes: HashMap<String, AltQuote> }
#[derive(Deserialize, Debug, Clone)]
struct AltQuote { #[serde(rename = "percentage_change_1h")] change_1h: f64, #[serde(rename = "percentage_change_24h")] change_24h: f64 }
#[derive(Deserialize, Debug, Clone)]
struct FngResponse { data: Vec<FngData> }
#[derive(Deserialize, Debug, Clone)]
struct FngData { value: String, value_classification: String }
#[derive(Deserialize, Debug, Clone)]
struct DexResponse { pairs: Option<Vec<DexPair>> }
#[derive(Deserialize, Debug, Clone)]
struct DexPair { #[serde(rename = "baseToken")] base_token: DexToken, price_usd: String, url: String }
#[derive(Deserialize, Debug, Clone)]
struct DexToken { name: String, symbol: String, address: String }
#[derive(Deserialize, Debug)]
struct JupiterQuoteResponse { #[serde(rename = "outAmount")] out_amount: String, #[serde(rename = "quoteResponse")] quote_response: Option<serde_json::Value> }
#[derive(Serialize)]
struct JupiterSwapRequest { #[serde(rename = "userPublicKey")] user_public_key: String, #[serde(rename = "quoteResponse")] quote_response: serde_json::Value }
#[derive(Deserialize)]
struct JupiterSwapResponse { #[serde(rename = "swapTransaction")] swap_transaction: String }

// APP STATE CONTAINER
struct AppState {
    states: Mutex<HashMap<ChatId, UserState>>,
    portfolios: Mutex<HashMap<ChatId, UserPortfolio>>,
    watchlist: Mutex<HashMap<ChatId, Vec<String>>>,
    users: Mutex<HashSet<ChatId>>,
    banned: Mutex<HashSet<ChatId>>,
}

// ==========================================
// 2. MODERN UI TEXT TEMPLATES
// ==========================================

fn get_welcome_text(name: &str) -> String {
    format!(
        "👋 <b>Welcome back, {}</b>\n\n\
        💠 <b>LUBIX TERMINAL v9.5</b>\n\
        <i>The Ultimate Financial Suite</i>\n\
        ━━━━━━━━━━━━━━━━━━━━━━━━\n\
        📊 <b>SYSTEM OVERVIEW</b>\n\
        ┌ 🟢 <b>Engine:</b>  <code>ONLINE</code>\n\
        ├ 🟣 <b>Solana:</b>  <code>SYNCED</code>\n\
        └ 🟠 <b>Data:</b>    <code>REAL-TIME</code>\n\n\
        🔐 <b>ACCESS LEVEL:</b> <code>TRADER</code>\n\
        👨‍💻 <b>DEV:</b> Iqbal (11 RPL IDN)\n\
        ━━━━━━━━━━━━━━━━━━━━━━━━\n\
        <i>Select a module to launch:</i>", 
        name
    )
}

fn get_crypto_info_text() -> String {
    "🪙 <b>CRYPTO INTELLIGENCE</b>\n\
    ━━━━━━━━━━━━━━━━━━━━━━━━\n\
    🌐 <b>GLOBAL MARKET</b>\n\
    • <b>Source:</b> <code>CoinMarketCap Pro</code>\n\
    • <b>Pair:</b>   <code>USD / IDR</code>\n\
    • <b>Status:</b> 🟢 <code>LIVE</code>\n\n\
    🔭 <b>ASSET EXPLORER</b>\n\
    Silakan masukkan <b>Ticker</b> (Simbol).\n\
    <i>Contoh:</i> <code>BTC</code>, <code>ETH</code>, <code>SOL</code>\n\
    ━━━━━━━━━━━━━━━━━━━━━━━━".to_string()
}

fn get_stock_info_text() -> String {
    "🕌 <b>ISSI STOCK SCREENER</b>\n\
    ━━━━━━━━━━━━━━━━━━━━━━━━\n\
    🇮🇩 <b>BURSA EFEK INDONESIA</b>\n\
    • <b>Index:</b>  <code>ISSI / JII</code>\n\
    • <b>Data:</b>   <code>Sharia & Fundamental</code>\n\
    • <b>Delay:</b>  <code>Real-time</code>\n\n\
    🔎 <b>EMITEN SEARCH</b>\n\
    Silakan masukkan <b>4 Digit Kode</b>.\n\
    <i>Contoh:</i> <code>BBRI</code>, <code>TLKM</code>, <code>ADRO</code>\n\
    ━━━━━━━━━━━━━━━━━━━━━━━━".to_string()
}

fn get_solana_info_text() -> String {
    "⚡️ <b>SOLANA ON-CHAIN</b>\n\
    ━━━━━━━━━━━━━━━━━━━━━━━━\n\
    🟣 <b>NETWORK STATUS</b>\n\
    • <b>RPC:</b>    <code>Mainnet-Beta</code>\n\
    • <b>DEX:</b>    <code>Raydium / Jupiter</code>\n\n\
    🧬 <b>TOKEN SCANNER</b>\n\
    Masukkan <b>Ticker</b> atau <b>CA</b>.\n\
    <i>Contoh:</i> <code>VETI</code> atau <code>So11...</code>\n\
    ━━━━━━━━━━━━━━━━━━━━━━━━".to_string()
}

fn get_sim_info_text() -> String {
    "🎮 <b>TRADING SIMULATOR PRO</b>\n\
    ━━━━━━━━━━━━━━━━━━━━━━━━\n\
    💼 <b>VIRTUAL ACCOUNT</b>\n\
    • <b>Capital:</b>  <code>$10,000.00</code>\n\
    • <b>Mode:</b>     <code>Spot Market</code>\n\
    • <b>Risk:</b>     <code>0% (Paper Trading)</code>\n\n\
    👇 <b>EXECUTE ORDER:</b>\n\
    ━━━━━━━━━━━━━━━━━━━━━━━━".to_string()
}

fn get_admin_text() -> String {
    "⚠️ <b>GOD MODE DASHBOARD</b> ⚠️\n\
    ━━━━━━━━━━━━━━━━━━━━━━━━\n\
    👤 <b>ADMIN:</b> Iqbal\n\
    🔐 <b>PERMS:</b>  <code>ROOT_ACCESS</code>\n\n\
    🎛 <b>CONTROL CENTER</b>\n\
    Manage users and system below.\n\
    ━━━━━━━━━━━━━━━━━━━━━━━━".to_string()
}

fn get_help_text() -> String {
    "📚 <b>LUBIX DOCUMENTATION</b>\n\
    ━━━━━━━━━━━━━━━━━━━━━━━━\n\
    🤖 <b>COMMAND LIST</b>\n\
    ┌ <code>/start</code>   ▶ Restart Terminal\n\
    ├ <code>/kripto</code>  ▶ Cek Harga Crypto\n\
    ├ <code>/saham</code>   ▶ Cek Saham Syariah\n\
    ├ <code>/solana</code>  ▶ Solana Scanner\n\
    ├ <code>/realbuy</code> ▶ Real Swap (Sol)\n\
    ├ <code>/sim</code>     ▶ Simulator Trading\n\
    └ <code>/panel</code>   ▶ Admin God Mode\n\n\
    📞 <b>SUPPORT</b>\n\
    Dev: @herebou\n\
    ━━━━━━━━━━━━━━━━━━━━━━━━".to_string()
}

// ==========================================
// 3. FUNGSI HELPER & LOGIC
// ==========================================

fn format_angka(n: f64) -> String { let s = format!("{:.0}", n); let mut res = String::new(); let len = s.len(); for (i, c) in s.chars().enumerate() { if i > 0 && (len - i) % 3 == 0 { res.push('.'); } res.push(c); } res }
fn format_money(n: f64) -> String { let s = format!("{:.2}", n); let mut res = String::new(); let parts: Vec<&str> = s.split('.').collect(); let int_part = parts[0]; let dec_part = parts[1]; let len = int_part.len(); for (i, c) in int_part.chars().enumerate() { if i > 0 && (len - i) % 3 == 0 { res.push(','); } res.push(c); } format!("{}.{}", res, dec_part) }
fn handle_na_value(val: &serde_json::Value) -> String { match val { serde_json::Value::Number(n) => format!("{:.2}", n.as_f64().unwrap_or(0.0)), _ => "N/A".to_string() } }
fn ticker_to_slug(t: &str) -> String { match t.to_uppercase().as_str() { "BTC" => "bitcoin".to_string(), "ETH" => "ethereum".to_string(), "SOL" => "solana".to_string(), "BNB" => "binance-coin".to_string(), _ => t.to_lowercase() } }
fn is_admin(chat_id: i64) -> bool { chat_id == 6187914291 }

fn get_bot_real_balance() -> String {
    let rpc_url = env::var("SOLANA_RPC_URL").unwrap_or("https://api.mainnet-beta.solana.com".to_string());
    let private_key_str = env::var("SOLANA_PRIVATE_KEY").unwrap_or_default();
    if private_key_str.is_empty() { return "No Wallet Configured".to_string(); }
    let keypair = Keypair::from_base58_string(&private_key_str); 
    let client = RpcClient::new(rpc_url);
    let pubkey = keypair.pubkey();
    if let Ok(bal) = client.get_balance(&pubkey) { let sol = bal as f64 / 1_000_000_000.0; return format!("🟣 <b>SOLANA WALLET</b>\n━━━━━━━━━━━━━━━━\n💰 <b>Balance:</b> <code>{:.4} SOL</code>\n🔑 <b>Address:</b>\n<code>{}</code>", sol, pubkey); }
    "Error Fetching Balance".to_string()
}

// --- API ---
async fn get_sharia_detail(ticker: &str) -> Result<EmitenDetail, Box<dyn std::error::Error + Send + Sync>> { let url = format!("https://syariahsaham-api.fly.dev/emiten/{}", ticker.to_uppercase()); let resp = reqwest::get(url).await?.json::<EmitenDetail>().await?; Ok(resp) }
async fn get_crypto_price_all(symbol: &str) -> Result<(CryptoData, f64), Box<dyn std::error::Error + Send + Sync>> { let api_key = env::var("CMC_API_KEY").unwrap_or_default(); let url = format!("https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?symbol={}&convert=USD", symbol.to_uppercase()); let client = reqwest::Client::new(); let resp = client.get(url).header("X-CMC_PRO_API_KEY", api_key).send().await?.json::<CmcResponse>().await?; let data = resp.data.get(&symbol.to_uppercase()).ok_or("Not Found")?.clone(); let usd = data.quote.get("USD").unwrap().price; Ok((data, usd * 16000.0)) }
async fn get_market_pulse(input: &str) -> Result<AltTickerData, Box<dyn std::error::Error + Send + Sync>> { let slug = ticker_to_slug(input); let url = format!("https://api.alternative.me/v2/ticker/{}/", slug); let resp = reqwest::get(url).await?.json::<AltTickerResponse>().await?; if let Some(data_map) = resp.data { return Ok(data_map.values().next().ok_or("Coin Not Found")?.clone()); } Err("API Error".into()) }
async fn get_fear_and_greed() -> Result<FngData, Box<dyn std::error::Error + Send + Sync>> { let url = "https://api.alternative.me/fng/"; let resp = reqwest::get(url).await?.json::<FngResponse>().await?; Ok(resp.data[0].clone()) }
async fn get_solana_dex_data(query: &str) -> Result<DexPair, Box<dyn std::error::Error + Send + Sync>> { let url = format!("https://api.dexscreener.com/latest/dex/search?q={}", query); let resp = reqwest::get(url).await?.json::<DexResponse>().await?; let pair = resp.pairs.and_then(|p| p.into_iter().find(|pair| pair.url.contains("solana"))).ok_or("Pair Not Found")?; Ok(pair) }
async fn execute_real_swap(output_mint: &str, amount_sol: f64) -> Result<String, Box<dyn std::error::Error + Send + Sync>> { let rpc_url = env::var("SOLANA_RPC_URL").unwrap_or("https://api.mainnet-beta.solana.com".to_string()); let private_key_str = env::var("SOLANA_PRIVATE_KEY").expect("Private Key Missing"); let rpc_client = RpcClient::new(rpc_url); let keypair = Keypair::from_base58_string(&private_key_str); let user_pubkey = keypair.pubkey().to_string(); let amount_lamports = (amount_sol * 1_000_000_000.0) as u64; let quote_url = format!("https://quote-api.jup.ag/v6/quote?inputMint=So11111111111111111111111111111111111111112&outputMint={}&amount={}&slippageBps=50", output_mint, amount_lamports); let client = reqwest::Client::new(); let quote_res = client.get(&quote_url).send().await?.json::<serde_json::Value>().await?; if quote_res.get("error").is_some() { return Err("Quote Failed".into()); } let swap_req = JupiterSwapRequest { user_public_key: user_pubkey.clone(), quote_response: quote_res.clone() }; let swap_res = client.post("https://quote-api.jup.ag/v6/swap").json(&swap_req).send().await?.json::<JupiterSwapResponse>().await?; let versioned_tx_bytes = general_purpose::STANDARD.decode(&swap_res.swap_transaction)?; let versioned_tx: solana_sdk::transaction::VersionedTransaction = bincode::deserialize(&versioned_tx_bytes)?; let signed_tx = solana_sdk::transaction::VersionedTransaction::try_new(versioned_tx.message, &[&keypair])?; let signature = rpc_client.send_and_confirm_transaction(&signed_tx)?; Ok(signature.to_string()) }

// ==========================================
// 4. KEYBOARDS
// ==========================================

fn make_main_menu() -> InlineKeyboardMarkup { InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback("🪙 CRYPTO", "menu_crypto"), InlineKeyboardButton::callback("🕌 SAHAM", "menu_sharia")], vec![InlineKeyboardButton::callback("⚡️ SOLANA DEX", "menu_solana"), InlineKeyboardButton::callback("🌡 SENTIMENT", "menu_sentiment_info")], vec![InlineKeyboardButton::callback("🎮 SIMULATOR", "menu_sim_main")], vec![InlineKeyboardButton::callback("🚀 REAL BUY", "menu_solana_real"), InlineKeyboardButton::callback("⭐ WATCHLIST", "menu_watchlist")], vec![InlineKeyboardButton::callback("❓ HELP", "menu_help"), InlineKeyboardButton::callback("🔄 REFRESH", "back_to_main")]]) }
fn make_sim_menu() -> InlineKeyboardMarkup { InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback("📈 BUY ($1000)", "menu_buy"), InlineKeyboardButton::callback("📉 SELL (ALL)", "menu_sell")], vec![InlineKeyboardButton::callback("💼 PORTFOLIO", "menu_portfolio")], vec![InlineKeyboardButton::callback("🔙 BACK", "back_to_main")]]) }
fn make_admin_menu() -> InlineKeyboardMarkup { InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback("📊 STATS", "admin_stats"), InlineKeyboardButton::callback("💰 WALLET", "admin_wallet")], vec![InlineKeyboardButton::callback("🎁 GIFT", "admin_gift"), InlineKeyboardButton::callback("📩 DM", "admin_dm")], vec![InlineKeyboardButton::callback("🚫 BAN", "admin_ban"), InlineKeyboardButton::callback("✅ UNBAN", "admin_unban")], vec![InlineKeyboardButton::callback("📢 BROADCAST", "admin_broadcast"), InlineKeyboardButton::callback("❌ CLOSE", "back_to_main")]]) }
fn make_result_footer() -> InlineKeyboardMarkup { InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback("🏠 HOME", "back_to_main"), InlineKeyboardButton::callback("➕ SAVE", "add_watchlist")]]) }

// ==========================================
// 5. MAIN
// ==========================================

fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(4 * 1024 * 1024).build().unwrap();

    runtime.block_on(async {
        println!("LubixBot v9.5 FINAL COMMANDS ONLINE 🚀");
        let bot = Bot::from_env();
        
        // [FIX UTAMA] REGISTER SEMUA COMMAND BIAR MUNCUL DI MENU TELEGRAM
        let commands = vec![
            BotCommand::new("start", "🏠 Dashboard Utama"),
            BotCommand::new("kripto", "🪙 Cek Harga Crypto"),
            BotCommand::new("saham", "🕌 Cek Saham Syariah"),
            BotCommand::new("solana", "⚡️ Solana Scanner"),
            BotCommand::new("realbuy", "🚀 Real Buy (Sol)"),
            BotCommand::new("sim", "🎮 Trading Simulator"),
            BotCommand::new("panel", "🔐 Admin Panel"),
            BotCommand::new("help", "❓ Bantuan"),
        ];
        let _ = bot.set_my_commands(commands).await;

        let app_state = Arc::new(AppState {
            states: Mutex::new(HashMap::new()),
            portfolios: Mutex::new(HashMap::new()),
            watchlist: Mutex::new(HashMap::new()),
            users: Mutex::new(HashSet::new()),
            banned: Mutex::new(HashSet::new()),
        });

        let handler = dptree::entry().branch(Update::filter_message().endpoint(message_handler)).branch(Update::filter_callback_query().endpoint(callback_handler));
        Dispatcher::builder(bot, handler).dependencies(dptree::deps![app_state]).enable_ctrlc_handler().build().dispatch().await;
    });
}

// ==========================================
// 6. HANDLERS
// ==========================================

async fn message_handler(bot: Bot, msg: Message, state: Arc<AppState>) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    let text = msg.text().unwrap_or("");

    if !is_admin(chat_id.0) {
        let banned = state.banned.lock().await;
        if banned.contains(&chat_id) { 
            bot.send_message(chat_id, "🚫 <b>ACCESS DENIED</b>\nYou are banned by Administrator.").parse_mode(ParseMode::Html).await?; 
            return Ok(()); 
        }
    }
    { let mut users = state.users.lock().await; users.insert(chat_id); }

    let mut state_lock = state.states.lock().await;
    
    // [FIX] HANDLE COMMANDS DIRECTLY
    match text {
        "/start" => { state_lock.insert(chat_id, UserState::Idle); bot.send_message(chat_id, get_welcome_text(&msg.from().as_ref().unwrap().first_name)).parse_mode(ParseMode::Html).reply_markup(make_main_menu()).await?; return Ok(()); }
        "/sim" => { state_lock.insert(chat_id, UserState::Idle); bot.send_message(chat_id, get_sim_info_text()).parse_mode(ParseMode::Html).reply_markup(make_sim_menu()).await?; return Ok(()); }
        "/kripto" => { state_lock.insert(chat_id, UserState::AwaitingCrypto); bot.send_message(chat_id, get_crypto_info_text()).parse_mode(ParseMode::Html).await?; return Ok(()); }
        "/saham" => { state_lock.insert(chat_id, UserState::AwaitingStock); bot.send_message(chat_id, get_stock_info_text()).parse_mode(ParseMode::Html).await?; return Ok(()); }
        "/solana" => { state_lock.insert(chat_id, UserState::AwaitingSolanaTicker); bot.send_message(chat_id, get_solana_info_text()).parse_mode(ParseMode::Html).await?; return Ok(()); }
        "/realbuy" => { state_lock.insert(chat_id, UserState::AwaitingRealBuyCA); bot.send_message(chat_id, "⚠️ <b>REAL MONEY MODE</b>\nInput Token CA:").parse_mode(ParseMode::Html).await?; return Ok(()); }
        "/panel" => { if is_admin(chat_id.0) { state_lock.insert(chat_id, UserState::Idle); bot.send_message(chat_id, get_admin_text()).parse_mode(ParseMode::Html).reply_markup(make_admin_menu()).await?; } else { bot.send_message(chat_id, "❓ Unknown").await?; } return Ok(()); }
        "/help" => { bot.send_message(chat_id, get_help_text()).parse_mode(ParseMode::Html).reply_markup(make_main_menu()).await?; return Ok(()); }
        _ => {}
    }

    let current = state_lock.get(&chat_id).cloned().unwrap_or(UserState::Idle);
    match (current, text) {
        // [FULL UI] SAHAM
        (UserState::AwaitingStock, t) => {
            if let Ok(e) = get_sharia_detail(t).await {
                let res = format!(
                    "🏢 <b>{} ({})</b>\n\
                    <i>{} | {}</i>\n\
                    ━━━━━━━━━━━━━━━━━━━━━━━━\n\
                    💰 <b>PRICE ACTION</b>\n\
                    🏷 <b>Price:</b> <code>Rp {}</code>\n\
                    📈 <b>Trend:</b> {}{}\n\n\
                    🕌 <b>SHARIA STATUS</b>\n\
                    ⚖️ <b>ISSI:</b> {}\n\
                    💸 <b>Debt:</b> <code>{}%</code>\n\
                    🚫 <b>Riba:</b> <code>{}%</code>\n\n\
                    📊 <b>KEY METRICS</b>\n\
                    📦 <b>Mkt Cap:</b> <code>Rp {}</code>\n\
                    📑 <b>Shares:</b>  <code>{}</code>\n\n\
                    🏭 <b>PROFILE</b>\n\
                    • Papan: {}\n\
                    • Index: {}\n\
                    • IPO:   {}\n\
                    ━━━━━━━━━━━━━━━━━━━━━━━━", 
                    e.nama.to_uppercase(), e.kode, e.sector, e.industry,
                    format_angka(e.harga.now as f64), if e.harga.change > 0 { "+" } else { "" }, e.harga.change, 
                    if e.issi { "✅ COMPLIANT" } else { "❌ NON-COMPLIANT" }, 
                    handle_na_value(&e.syariah_indicator.hutang_bunga), handle_na_value(&e.syariah_indicator.non_halal),
                    format_angka(e.market_cap), format_angka(e.shares),
                    e.papan, e.index, e.ipo
                );
                bot.send_message(chat_id, res).parse_mode(ParseMode::Html).reply_markup(make_result_footer()).await?;
            } else { bot.send_message(chat_id, "❌ Emiten Not Found.").await?; }
            state_lock.insert(chat_id, UserState::Idle);
        }
        // [FULL UI] CRYPTO
        (UserState::AwaitingCrypto, t) => {
            if let Ok((d, idr)) = get_crypto_price_all(t).await {
                let q = d.quote.get("USD").unwrap();
                let arrow_1h = if q.change_1h >= 0.0 { "↗️" } else { "↘️" };
                let arrow_24h = if q.change_24h >= 0.0 { "↗️" } else { "↘️" };
                let arrow_7d = if q.change_7d >= 0.0 { "↗️" } else { "↘️" };

                let res = format!(
                    "🪙 <b>{} ({})</b>\n\
                    ━━━━━━━━━━━━━━━━━━━━━━━━\n\
                    💵 <b>VALUATION</b>\n\
                    🇺🇸 <code>$ {}</code>\n\
                    🇮🇩 <code>Rp {}</code>\n\n\
                    📊 <b>PERFORMANCE</b>\n\
                    • 1H:  <code>{:.2}%</code> {}\n\
                    • 24H: <code>{:.2}%</code> {}\n\
                    • 7D:  <code>{:.2}%</code> {}\n\n\
                    💎 <b>MARKET DATA</b>\n\
                    • Cap: <code>${}</code>\n\
                    • Vol: <code>${}</code>\n\
                    ━━━━━━━━━━━━━━━━━━━━━━━━",
                    d.name, d.symbol, 
                    format_money(q.price), format_angka(idr),
                    q.change_1h, arrow_1h, q.change_24h, arrow_24h, q.change_7d, arrow_7d,
                    format_money(q.market_cap), format_money(q.volume_24h)
                );
                bot.send_message(chat_id, res).parse_mode(ParseMode::Html).reply_markup(make_result_footer()).await?;
            } else { bot.send_message(chat_id, "❌ Coin Not Found.").await?; }
            state_lock.insert(chat_id, UserState::Idle);
        }
        // [ADMIN]
        (UserState::AwaitingBanUser, t) => { 
            if is_admin(chat_id.0) { if let Ok(id) = t.trim().parse::<i64>() { state.banned.lock().await.insert(ChatId(id)); bot.send_message(chat_id, format!("🚫 <b>SUCCESS:</b> User <code>{}</code> BANNED.", id)).parse_mode(ParseMode::Html).await?; } else { bot.send_message(chat_id, "❌ Invalid ID").await?; } } 
            state_lock.insert(chat_id, UserState::Idle); 
        }
        (UserState::AwaitingUnbanUser, t) => { 
            if is_admin(chat_id.0) { if let Ok(id) = t.trim().parse::<i64>() { state.banned.lock().await.remove(&ChatId(id)); bot.send_message(chat_id, format!("✅ <b>SUCCESS:</b> User <code>{}</code> UNBANNED.", id)).parse_mode(ParseMode::Html).await?; } else { bot.send_message(chat_id, "❌ Invalid ID").await?; } } 
            state_lock.insert(chat_id, UserState::Idle); 
        }
        (UserState::AwaitingGiftSaldo, t) => { 
            if is_admin(chat_id.0) {
                let p: Vec<&str> = t.trim().split_whitespace().collect();
                if p.len() == 2 {
                    if let (Ok(id), Ok(amt)) = (p[0].parse::<i64>(), p[1].parse::<f64>()) {
                        let mut port = state.portfolios.lock().await;
                        let u = port.entry(ChatId(id)).or_insert(UserPortfolio{balance:10000.0, holdings:HashMap::new()});
                        u.balance += amt;
                        bot.send_message(chat_id, format!("🎁 <b>SENT:</b> ${} to {}", amt, id)).parse_mode(ParseMode::Html).await?;
                        let _ = bot.send_message(ChatId(id), format!("🎁 <b>GIFT RECEIVED!</b>\nAmount: <code>${}</code>", amt)).parse_mode(ParseMode::Html).await;
                    }
                }
            }
            state_lock.insert(chat_id, UserState::Idle);
        }
        (UserState::AwaitingBroadcast, t) => {
            if is_admin(chat_id.0) {
                let u = state.users.lock().await;
                for &id in u.iter() { if id != chat_id { let _ = bot.send_message(id, format!("📢 <b>ANNOUNCEMENT</b>\n\n{}", t)).parse_mode(ParseMode::Html).await; } }
                bot.send_message(chat_id, format!("✅ Broadcast to {} users", u.len()-1)).await?;
            }
            state_lock.insert(chat_id, UserState::Idle);
        }
        (UserState::AwaitingDirectMsg, t) => {
            if is_admin(chat_id.0) {
                if let Some((id_str, msg)) = t.split_once(' ') {
                    if let Ok(id) = id_str.parse::<i64>() {
                        let _ = bot.send_message(ChatId(id), format!("📩 <b>ADMIN MESSAGE:</b>\n\n{}", msg)).parse_mode(ParseMode::Html).await;
                        bot.send_message(chat_id, "✅ DM Sent").await?;
                    }
                }
            }
            state_lock.insert(chat_id, UserState::Idle);
        }
        
        // [SIMULATOR]
        (UserState::AwaitingBuyTicker, t) => {
            if let Ok((d, _)) = get_crypto_price_all(t).await {
                let p = d.quote.get("USD").unwrap().price;
                let mut port = state.portfolios.lock().await;
                let u = port.entry(chat_id).or_insert(UserPortfolio{balance:10000.0, holdings:HashMap::new()});
                if u.balance >= 1000.0 {
                    u.balance -= 1000.0;
                    let h = u.holdings.entry(t.to_uppercase()).or_insert(Holding{symbol:t.to_uppercase(), quantity:0.0, avg_price:0.0});
                    let cost = (h.quantity * h.avg_price) + 1000.0; h.quantity += 1000.0/p; h.avg_price = cost/h.quantity;
                    bot.send_message(chat_id, format!("📝 <b>ORDER EXECUTED</b>\n━━━━━━━━━━━━━━━━━━━━━━━━\n🟢 <b>BUY FILLED</b>\n📦 Asset: <b>{}</b>\n💲 Price: <code>${:.2}</code>\n💰 Cost:  <code>$1,000.00</code>\n━━━━━━━━━━━━━━━━━━━━━━━━\n💼 <b>New Bal:</b> <code>${:.2}</code>", t.to_uppercase(), p, u.balance)).parse_mode(ParseMode::Html).reply_markup(make_sim_menu()).await?;
                } else { bot.send_message(chat_id, "❌ <b>INSUFFICIENT FUNDS</b>").parse_mode(ParseMode::Html).await?; }
            } else { bot.send_message(chat_id, "❌ Invalid Ticker").await?; }
            state_lock.insert(chat_id, UserState::Idle);
        }
        (UserState::AwaitingSellTicker, t) => {
            if let Ok((d, _)) = get_crypto_price_all(t).await {
                let p = d.quote.get("USD").unwrap().price;
                let mut port = state.portfolios.lock().await;
                let u = port.entry(chat_id).or_insert(UserPortfolio{balance:10000.0, holdings:HashMap::new()});
                if let Some(h) = u.holdings.get(&t.to_uppercase()) {
                    let val = h.quantity * p;
                    let pnl = val - (h.quantity * h.avg_price);
                    let emoji = if pnl >= 0.0 { "🟢" } else { "🔴" };
                    u.balance += val; u.holdings.remove(&t.to_uppercase());
                    bot.send_message(chat_id, format!("📝 <b>ORDER EXECUTED</b>\n━━━━━━━━━━━━━━━━━━━━━━━━\n🔴 <b>SELL FILLED</b>\n📦 Asset: <b>{}</b>\n💵 Value: <code>${:.2}</code>\n{} <b>PnL:</b> <code>${:.2}</code>\n━━━━━━━━━━━━━━━━━━━━━━━━\n💼 <b>New Bal:</b> <code>${:.2}</code>", t.to_uppercase(), val, emoji, pnl, u.balance)).parse_mode(ParseMode::Html).reply_markup(make_sim_menu()).await?;
                } else { bot.send_message(chat_id, "❌ No Asset").await?; }
            } else { bot.send_message(chat_id, "❌ Error").await?; }
            state_lock.insert(chat_id, UserState::Idle);
        }
        
        // [OTHERS]
        (UserState::AwaitingRealBuyCA, t) => { if t.len()<30 { bot.send_message(chat_id, "❌ Invalid CA").await?; } else { bot.send_message(chat_id, format!("⚡️ Swapping <code>{}</code>...", t)).parse_mode(ParseMode::Html).await?; match execute_real_swap(t, 0.001).await { Ok(s) => bot.send_message(chat_id, format!("✅ <b>SUCCESS</b>\nTx: <a href='https://solscan.io/tx/{}'>View</a>", s)).parse_mode(ParseMode::Html).await?, Err(e) => bot.send_message(chat_id, format!("❌ Error: {}", e)).await? }; } state_lock.insert(chat_id, UserState::Idle); }
        (UserState::AwaitingSolanaTicker, t) => { if let Ok(p) = get_solana_dex_data(t).await { bot.send_message(chat_id, format!("⚡️ <b>SOLANA DEX</b>\nToken: <b>{}</b>\nPrice: <code>${}</code>\nCA: <code>{}</code>", p.base_token.name, p.price_usd, p.base_token.address)).parse_mode(ParseMode::Html).await?; } else { bot.send_message(chat_id, "❌ Not Found").await?; } state_lock.insert(chat_id, UserState::Idle); }
        (UserState::AwaitingAddWatchlist, t) => { state.watchlist.lock().await.entry(chat_id).or_insert(Vec::new()).push(t.to_uppercase()); bot.send_message(chat_id, "✅ Added").await?; state_lock.insert(chat_id, UserState::Idle); }
        (UserState::AwaitingSentiment, t) => { if let Ok(d) = get_market_pulse(t).await { bot.send_message(chat_id, format!("📈 <b>MARKET PULSE: {}</b>\n1H: {:.2}%", d.name, d.quotes["USD"].change_1h)).parse_mode(ParseMode::Html).await?; } else { bot.send_message(chat_id, "❌ Error").await?; } state_lock.insert(chat_id, UserState::Idle); }
        _ => {}
    }
    Ok(())
}

async fn callback_handler(bot: Bot, q: CallbackQuery, state: Arc<AppState>) -> ResponseResult<()> {
    if let (Some(data), Some(msg)) = (q.data, q.message) {
        let chat_id = msg.chat.id;
        
        if !is_admin(chat_id.0) { let b = state.banned.lock().await; if b.contains(&chat_id) { return Ok(()); } }

        let mut state_lock = state.states.lock().await;
        match data.as_str() {
            "menu_crypto" => { state_lock.insert(chat_id, UserState::AwaitingCrypto); bot.send_message(chat_id, get_crypto_info_text()).parse_mode(ParseMode::Html).await?; }
            "menu_sharia" => { state_lock.insert(chat_id, UserState::AwaitingStock); bot.send_message(chat_id, get_stock_info_text()).parse_mode(ParseMode::Html).await?; }
            "menu_solana" => { state_lock.insert(chat_id, UserState::AwaitingSolanaTicker); bot.send_message(chat_id, get_solana_info_text()).parse_mode(ParseMode::Html).await?; }
            "menu_solana_real" => { state_lock.insert(chat_id, UserState::AwaitingRealBuyCA); bot.send_message(chat_id, "⚠️ <b>REAL MONEY MODE</b>\nInput CA:").parse_mode(ParseMode::Html).await?; }
            "menu_sim_main" => { bot.send_message(chat_id, get_sim_info_text()).parse_mode(ParseMode::Html).reply_markup(make_sim_menu()).await?; }
            "menu_buy" => { state_lock.insert(chat_id, UserState::AwaitingBuyTicker); bot.send_message(chat_id, "📈 Input Ticker:").await?; }
            "menu_sell" => { state_lock.insert(chat_id, UserState::AwaitingSellTicker); bot.send_message(chat_id, "📉 Input Ticker:").await?; }
            "menu_portfolio" => { 
                let p = state.portfolios.lock().await; let u = p.get(&chat_id).cloned().unwrap_or(UserPortfolio{balance:10000.0, holdings:HashMap::new()});
                let mut res = format!("💼 <b>MY PORTFOLIO</b>\n━━━━━━━━━━━━━━━━━━━━━━━━\n💳 <b>Balance:</b> <code>${:.2}</code>\n\n📦 <b>ASSETS:</b>\n", u.balance);
                for h in u.holdings.values() { res.push_str(&format!("• <b>{}</b>: <code>{:.4}</code>\n", h.symbol, h.quantity)); }
                bot.send_message(chat_id, res).parse_mode(ParseMode::Html).reply_markup(make_sim_menu()).await?;
            }
            "admin_stats" => { 
                if is_admin(chat_id.0) { 
                    let u = state.users.lock().await; 
                    let mut s = format!("📊 <b>STATS</b>\nUsers: {}\n\nIDs:\n", u.len());
                    for id in u.iter() { s.push_str(&format!("<code>{}</code>\n", id)); }
                    bot.send_message(chat_id, s).parse_mode(ParseMode::Html).await?; 
                } 
            }
            "admin_wallet" => { if is_admin(chat_id.0) { bot.send_message(chat_id, get_bot_real_balance()).parse_mode(ParseMode::Html).await?; } }
            "admin_gift" => { if is_admin(chat_id.0) { state_lock.insert(chat_id, UserState::AwaitingGiftSaldo); bot.send_message(chat_id, "🎁 Input: <code>ID AMOUNT</code>").parse_mode(ParseMode::Html).await?; } }
            "admin_ban" => { if is_admin(chat_id.0) { state_lock.insert(chat_id, UserState::AwaitingBanUser); bot.send_message(chat_id, "🚫 Input ID").await?; } }
            "admin_unban" => { if is_admin(chat_id.0) { state_lock.insert(chat_id, UserState::AwaitingUnbanUser); bot.send_message(chat_id, "✅ Input ID").await?; } }
            "admin_dm" => { if is_admin(chat_id.0) { state_lock.insert(chat_id, UserState::AwaitingDirectMsg); bot.send_message(chat_id, "📩 Input: <code>ID MSG</code>").parse_mode(ParseMode::Html).await?; } }
            "admin_broadcast" => { if is_admin(chat_id.0) { state_lock.insert(chat_id, UserState::AwaitingBroadcast); bot.send_message(chat_id, "📢 Input Msg").await?; } }
            "menu_watchlist" => { let w = state.watchlist.lock().await; let l = w.get(&chat_id).cloned().unwrap_or_default(); bot.send_message(chat_id, format!("⭐ <b>WATCHLIST</b>\n{:?}", l)).parse_mode(ParseMode::Html).reply_markup(make_main_menu()).await?; }
            "add_watchlist" => { state_lock.insert(chat_id, UserState::AwaitingAddWatchlist); bot.send_message(chat_id, "⭐ Input:").await?; }
            "menu_sentiment_info" => { bot.send_message(chat_id, "🌡 <b>SENTIMENT</b>").reply_markup(InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback("🎭 F&G Index", "menu_fng"), InlineKeyboardButton::callback("📈 PULSE", "menu_pulse")]])).await?; }
            "menu_fng" => { if let Ok(f) = get_fear_and_greed().await { bot.send_message(chat_id, format!("🎭 <b>F&G: {}</b>", f.value_classification)).parse_mode(ParseMode::Html).await?; } }
            "menu_pulse" => { state_lock.insert(chat_id, UserState::AwaitingSentiment); bot.send_message(chat_id, "📈 Input Ticker:").await?; }
            "menu_help" => { bot.send_message(chat_id, get_help_text()).parse_mode(ParseMode::Html).reply_markup(make_main_menu()).await?; }
            "back_to_main" => { state_lock.insert(chat_id, UserState::Idle); bot.send_message(chat_id, get_welcome_text(&q.from.first_name)).parse_mode(ParseMode::Html).reply_markup(make_main_menu()).await?; }
            _ => {}
        }
    }
    bot.answer_callback_query(q.id).await?;
    Ok(())
}