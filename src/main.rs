use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode, BotCommand};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::env;

// --- 1. STRUKTUR DATA ---
#[derive(Clone, PartialEq)]
enum UserState { 
    Idle, AwaitingCrypto, AwaitingStock, AwaitingSentiment, 
    AwaitingBuyTicker, AwaitingSellTicker, AwaitingAddWatchlist 
}

#[derive(Debug, Clone)]
struct Holding { symbol: String, quantity: f64, avg_price: f64 }

#[derive(Debug, Clone)]
struct UserPortfolio { balance: f64, holdings: HashMap<String, Holding> }

#[derive(Deserialize, Debug, Clone)]
struct SyariahIndicator {
    #[serde(rename = "hutangBunga")] hutang_bunga: serde_json::Value, 
    #[serde(rename = "nonHalal")] non_halal: serde_json::Value,      
    business: bool, 
}

#[derive(Deserialize, Debug, Clone)]
struct StockPrice { now: i64, #[serde(rename = "deltaPrice")] change: i64 }

#[derive(Deserialize, Debug, Clone)]
struct EmitenDetail {
    #[serde(rename = "code")] kode: String, #[serde(rename = "name")] nama: String, 
    sector: String, industry: String, ipo: String, papan: String, index: String,
    #[serde(rename = "marketCap")] market_cap: f64, shares: f64, issi: bool,
    #[serde(rename = "syariahIndicator")] syariah_indicator: SyariahIndicator,
    harga: StockPrice,
}

#[derive(Deserialize, Debug, Clone)]
struct CmcResponse { data: HashMap<String, CryptoData> }
#[derive(Deserialize, Debug, Clone)]
struct CryptoData { name: String, symbol: String, quote: HashMap<String, QuoteData> }
#[derive(Deserialize, Debug, Clone)]
struct QuoteData { price: f64 }

#[derive(Deserialize, Debug, Clone)]
struct AltTickerResponse { data: Option<HashMap<String, AltTickerData>> }
#[derive(Deserialize, Debug, Clone)]
struct AltTickerData { name: String, symbol: String, quotes: HashMap<String, AltQuote> }
#[derive(Deserialize, Debug, Clone)]
struct AltQuote {
    #[serde(rename = "percentage_change_1h")] change_1h: f64,
    #[serde(rename = "percentage_change_24h")] change_24h: f64,
    #[serde(rename = "percentage_change_7d")] change_7d: f64,
}

#[derive(Deserialize, Debug, Clone)]
struct FngResponse { data: Vec<FngData> }
#[derive(Deserialize, Debug, Clone)]
struct FngData { value: String, value_classification: String }

type PortfolioStore = Arc<Mutex<HashMap<ChatId, UserPortfolio>>>;
type WatchlistStore = Arc<Mutex<HashMap<ChatId, Vec<String>>>>;

// --- 2. TEMPLATE UI ASLI (DIRESTORE TOTAL) ---

fn get_welcome_text(name: &str) -> String {
    format!(
        "💎 <b>LUBIX TERMINAL v3.7</b> 💎\n\
        <i>Professional Financial Co-Pilot</i>\n\
        ━━━━━━━━━━━━━━━━━━━━━\n\n\
        Selamat datang, <b>{}</b>.\n\
        LubixBot adalah sistem integrasi data pasar modal dan crypto yang dibangun khusus untuk memberikan wawasan investasi yang cerdas.\n\n\
        🚀 <b>SYSTEM STATUS:</b>\n\
        • 🛰 <b>Network:</b> <code>CONNECTED</code>\n\
        • 📈 <b>Market:</b> <code>OPEN (Real-time Mode)</code>\n\
        • 🕌 <b>Syariah Data:</b> <code>SYNCED (Fly.dev API)</code>\n\n\
        👨‍💻 <b>DEVELOPER INFO:</b>\n\
        Siswa 11 RPL IDN Boarding School Solo. Menargetkan <b>School of Computing NUS 2026</b>.\n\n\
        ━━━━━━━━━━━━━━━━━━━━━\n\
        <i>Gunakan tombol di bawah atau ketik / untuk navigasi:</i>", 
        name
    )
}

fn get_crypto_info_text() -> String {
    "🪙 <b>MODUL CRYPTOCURRENCY ENGINE</b>\n\n\
    Akses data harga real-time dari <b>CoinMarketCap API</b>. Mendukung lebih dari 10.000+ aset digital global.\n\n\
    🛠 <b>INFO MODUL:</b>\n\
    • Konversi Otomatis ke <b>IDR</b> (Rp 16.000)\n\
    • Update Data: <code>Per 1 Menit</code>\n\
    • Format Output: <code>Ticker Prefix ($)</code>\n\n\
    📖 <b>PETUNJUK PENGGUNAAN:</b>\n\
    Ketik Ticker koin yang ingin dicari tanpa tanda baca.\n\n\
    <b>Contoh:</b> <code>BTC</code> atau <code>SOL</code>".to_string()
}

fn get_stock_info_text() -> String {
    "🕌 <b>MODUL SHARIA SCREENER IDX</b>\n\n\
    Sistem screening otomatis yang terhubung ke database <b>SyariahSaham API</b>. Membedah data emiten berdasarkan kriteria <b>ISSI (Indeks Saham Syariah Indonesia)</b>.\n\n\
    🛠 <b>INFO MODUL:</b>\n\
    • Analisis: <code>Debt Ratio & Non-Halal Rev</code>\n\
    • Cakupan: <code>Seluruh Emiten BEI (IDX)</code>\n\n\
    📖 <b>PETUNJUK PENGGUNAAN:</b>\n\
    Ketik Kode Emiten (4 huruf kapital).\n\n\
    <b>Contoh:</b> <code>BBRI</code> atau <code>MERI</code>".to_string()
}

fn get_sentiment_info_text() -> String {
    "🌡 <b>MODUL SENTIMENT ENGINE</b>\n\n\
    Modul ini menganalisis psikologi market melalui momentum harga dan index emosi market global.\n\n\
    🛠 <b>INFO MODUL:</b>\n\
    • Source: <b>Alternative.me Sentiment API</b>\n\
    • Feature: <b>Fear & Greed Index & Pulse Momentum</b>\n\n\
    📖 <b>PETUNJUK:</b>\n\
    Pilih <b>F&G Index</b> untuk emosi global atau <b>PULSE</b> untuk momentum spesifik koin.".to_string()
}

fn get_trading_info_text() -> String {
    "📈 <b>LUBIX VIRTUAL BROKER (BUY)</b>\n\n\
    Modul simulasi trading menggunakan saldo virtual. Memungkinkan Anda latihan tanpa risiko finansial.\n\n\
    🛠 <b>INFO TRADING:</b>\n\
    • Saldo Awal: <b>$10.000 (Virtual)</b>\n\
    • Order Size: <b>Fixed $1.000 per Trade</b>\n\n\
    📖 <b>PETUNJUK:</b> Masukkan ticker koin yang ingin dibeli.".to_string()
}

fn get_help_text() -> String {
    "❓ <b>LUBIX HELP & DOCUMENTATION</b>\n\
    <i>Panduan Lengkap Penggunaan Terminal</i>\n\
    ━━━━━━━━━━━━━━━━━━━━━\n\n\
    📖 <b>DAFTAR PERINTAH:</b>\n\
    • /start - Reset & Dashboard Utama\n\
    • /kripto - Modul Harga Crypto Global\n\
    • /saham - Modul Sharia Stock Indonesia\n\
    • /sentiment - Analisis Sentimen Market\n\
    • /help - Pusat Bantuan & Info Teknis\n\n\
    📬 <b>KONTAK ADMIN:</b> Hubungi @herebou\n\
    ━━━━━━━━━━━━━━━━━━━━━".to_string()
}

// --- 3. FUNGSI HELPER ---

fn format_angka(n: f64) -> String {
    let s = format!("{:.0}", n);
    let mut res = String::new();
    let len = s.len();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 { res.push('.'); }
        res.push(c);
    }
    res
}

fn handle_na_value(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::Number(n) => format!("{:.2}", n.as_f64().unwrap_or(0.0)),
        _ => "Data belum tersedia".to_string(),
    }
}

fn ticker_to_slug(t: &str) -> String {
    match t.to_uppercase().as_str() {
        "BTC" => "bitcoin".to_string(), "ETH" => "ethereum".to_string(),
        "SOL" => "solana".to_string(), "BNB" => "binance-coin".to_string(),
        _ => t.to_lowercase(),
    }
}

// --- API FUNCTIONS (SAFE) ---
async fn get_sharia_detail(ticker: &str) -> Result<EmitenDetail, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://syariahsaham-api.fly.dev/emiten/{}", ticker.to_uppercase());
    let resp = reqwest::get(url).await?.json::<EmitenDetail>().await?;
    Ok(resp)
}

async fn get_crypto_price_all(symbol: &str) -> Result<(CryptoData, f64), Box<dyn std::error::Error + Send + Sync>> {
    let api_key = env::var("CMC_API_KEY").unwrap_or_default();
    let url = format!("https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?symbol={}&convert=USD", symbol.to_uppercase());
    let client = reqwest::Client::new();
    let resp = client.get(url).header("X-CMC_PRO_API_KEY", api_key).send().await?.json::<CmcResponse>().await?;
    let data = resp.data.get(&symbol.to_uppercase()).ok_or("Not Found")?.clone();
    let usd = data.quote.get("USD").unwrap().price;
    Ok((data, usd * 16000.0))
}

async fn get_fear_and_greed() -> Result<FngData, Box<dyn std::error::Error + Send + Sync>> {
    let url = "https://api.alternative.me/fng/";
    let resp = reqwest::get(url).await?.json::<FngResponse>().await?;
    Ok(resp.data[0].clone())
}

async fn get_market_pulse(input: &str) -> Result<AltTickerData, Box<dyn std::error::Error + Send + Sync>> {
    let slug = ticker_to_slug(input);
    let url = format!("https://api.alternative.me/v2/ticker/{}/", slug);
    let resp = reqwest::get(url).await?.json::<AltTickerResponse>().await?;
    if let Some(data_map) = resp.data {
        return Ok(data_map.values().next().ok_or("Coin Not Found")?.clone());
    }
    Err("API Error".into())
}

// --- KEYBOARDS ---
fn make_main_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("💰 CRYPTO", "menu_crypto"), InlineKeyboardButton::callback("🕌 SAHAM", "menu_sharia")],
        vec![InlineKeyboardButton::callback("🌡 SENTIMENT", "menu_sentiment_info"), InlineKeyboardButton::callback("⭐ WATCHLIST", "menu_watchlist")],
        vec![InlineKeyboardButton::callback("📈 BUY", "menu_buy"), InlineKeyboardButton::callback("💼 PORTFOLIO", "menu_portfolio")],
        vec![InlineKeyboardButton::callback("❓ HELP", "menu_help"), InlineKeyboardButton::callback("🏠 HOME", "back_to_main")],
    ])
}

fn make_result_footer() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🏠 HOME", "back_to_main"), InlineKeyboardButton::callback("➕ ⭐", "add_watchlist"), InlineKeyboardButton::callback("📉 SELL", "menu_sell")],
    ])
}

// --- MAIN RUNTIME ---
fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(4 * 1024 * 1024).build().unwrap();

    runtime.block_on(async {
        println!("LubixBot v3.7 Final Restoration ONLINE 🚀");
        let bot = Bot::from_env();
        let commands = vec![BotCommand::new("start", "🏠 Dashboard"), BotCommand::new("kripto", "🪙 Crypto"), BotCommand::new("saham", "🕌 Saham"), BotCommand::new("help", "❓ Help")];
        let _ = bot.set_my_commands(commands).await;

        let user_states = Arc::new(Mutex::new(HashMap::<ChatId, UserState>::new()));
        let portfolios = Arc::new(Mutex::new(HashMap::<ChatId, UserPortfolio>::new()));
        let watchlist = Arc::new(Mutex::new(HashMap::<ChatId, Vec<String>>::new()));

        let handler = dptree::entry().branch(Update::filter_message().endpoint(message_handler)).branch(Update::filter_callback_query().endpoint(callback_handler));
        Dispatcher::builder(bot, handler).dependencies(dptree::deps![user_states, portfolios, watchlist]).enable_ctrlc_handler().build().dispatch().await;
    });
}

// --- HANDLERS ---
async fn message_handler(bot: Bot, msg: Message, states: Arc<Mutex<HashMap<ChatId, UserState>>>, portfolios: PortfolioStore, watchlist: WatchlistStore) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    let text = msg.text().unwrap_or("");
    let mut state_lock = states.lock().await;

    match text {
        "/start" => { state_lock.insert(chat_id, UserState::Idle); bot.send_message(chat_id, get_welcome_text(&msg.from.as_ref().unwrap().first_name)).parse_mode(ParseMode::Html).reply_markup(make_main_menu()).await?; return Ok(()); }
        "/help" => { bot.send_message(chat_id, get_help_text()).parse_mode(ParseMode::Html).reply_markup(make_main_menu()).await?; return Ok(()); }
        "/kripto" => { state_lock.insert(chat_id, UserState::AwaitingCrypto); bot.send_message(chat_id, get_crypto_info_text()).parse_mode(ParseMode::Html).await?; return Ok(()); }
        "/saham" => { state_lock.insert(chat_id, UserState::AwaitingStock); bot.send_message(chat_id, get_stock_info_text()).parse_mode(ParseMode::Html).await?; return Ok(()); }
        _ => {}
    }

    let current_state = state_lock.get(&chat_id).cloned().unwrap_or(UserState::Idle);
    match (current_state, text) {
        (UserState::AwaitingStock, ticker) if !ticker.starts_with('/') => {
            bot.send_message(chat_id, format!("⏳ <b>Scanning:</b> <code>{}</code>...", ticker.to_uppercase())).parse_mode(ParseMode::Html).await?;
            if let Ok(e) = get_sharia_detail(ticker).await {
                let response = format!(
                    "🏢 <b>{}</b>\n🔖 Ticker: <code>{}</code>\n━━━━━━━━━━━━━━━━━━━━━\n\n💰 <b>PRICE:</b> Rp {} ({}{})\n• Papan: <code>{}</code>\n• Index: <code>{}</code>\n\n🕌 <b>SHARIA:</b>\n• Status: <b>{}</b>\n• Debt: <code>{}</code>\n• Non-Halal: <code>{}</code>\n\n📊 <b>DATA:</b>\n• Market Cap: Rp {}\n• Sektor: {}\n━━━━━━━━━━━━━━━━━━━━━",
                    e.nama.to_uppercase(), e.kode, format_angka(e.harga.now as f64),
                    if e.harga.change > 0 { "+" } else { "" }, e.harga.change, e.papan, e.index,
                    if e.issi { "✅ Syariah" } else { "❌ Non-Syariah" },
                    handle_na_value(&e.syariah_indicator.hutang_bunga), handle_na_value(&e.syariah_indicator.non_halal),
                    format_angka(e.market_cap), e.sector
                );
                bot.send_message(chat_id, response).parse_mode(ParseMode::Html).reply_markup(make_result_footer()).await?;
            } else { bot.send_message(chat_id, "❌ Emiten tidak ditemukan.").await?; }
            state_lock.insert(chat_id, UserState::Idle);
        }
        (UserState::AwaitingSentiment, ticker) if !ticker.starts_with('/') => {
            bot.send_message(chat_id, format!("🌡 <b>Checking Pulse:</b> <code>{}</code>...", ticker.to_uppercase())).parse_mode(ParseMode::Html).await?;
            if let Ok(d) = get_market_pulse(ticker).await {
                let q = d.quotes.get("USD").unwrap();
                let res = format!("📈 <b>Momentum {} ({})</b>\n━━━━━━━━━━━━━━━━━━━━━\n1H: <code>{:.2}%</code>\n24H: <code>{:.2}%</code>\n7D: <code>{:.2}%</code>\n━━━━━━━━━━━━━━━━━━━━━", d.name, d.symbol, q.change_1h, q.change_24h, q.change_7d);
                bot.send_message(chat_id, res).parse_mode(ParseMode::Html).reply_markup(make_result_footer()).await?;
            } else { bot.send_message(chat_id, "❌ Data Pulse tidak ditemukan.").await?; }
            state_lock.insert(chat_id, UserState::Idle);
        }
        (UserState::AwaitingCrypto, ticker) if !ticker.starts_with('/') => {
            if let Ok((data, idr)) = get_crypto_price_all(ticker).await {
                let res = format!("🪙 <b>{} (${})</b>\n━━━━━━━━━━━━━━━━━━━━━\n💵 USD: ${:.2}\n🇮🇩 IDR: Rp {}\n━━━━━━━━━━━━━━━━━━━━━", data.name, data.symbol, data.quote.get("USD").unwrap().price, format_angka(idr));
                bot.send_message(chat_id, res).parse_mode(ParseMode::Html).reply_markup(make_result_footer()).await?;
            } else { bot.send_message(chat_id, "❌ Koin tidak ditemukan.").await?; }
            state_lock.insert(chat_id, UserState::Idle);
        }
        (UserState::AwaitingBuyTicker, ticker) if !ticker.starts_with('/') => {
            if let Ok((data, _)) = get_crypto_price_all(ticker).await {
                let price = data.quote.get("USD").unwrap().price;
                let mut port_lock = portfolios.lock().await;
                let p = port_lock.entry(chat_id).or_insert(UserPortfolio { balance: 10000.0, holdings: HashMap::new() });
                if p.balance >= 1000.0 {
                    p.balance -= 1000.0;
                    let h = p.holdings.entry(ticker.to_uppercase()).or_insert(Holding { symbol: ticker.to_uppercase(), quantity: 0.0, avg_price: 0.0 });
                    h.quantity += 1000.0 / price; h.avg_price = price;
                    bot.send_message(chat_id, format!("✅ Buy {} senilai $1000 Berhasil!", ticker.to_uppercase())).await?;
                } else { bot.send_message(chat_id, "❌ Saldo kurang.").await?; }
            }
            state_lock.insert(chat_id, UserState::Idle);
        }
        (UserState::AwaitingAddWatchlist, ticker) if !ticker.starts_with('/') => {
            watchlist.lock().await.entry(chat_id).or_insert(Vec::new()).push(ticker.to_uppercase());
            bot.send_message(chat_id, format!("✅ {} ditambahkan ke Watchlist!", ticker.to_uppercase())).await?;
            state_lock.insert(chat_id, UserState::Idle);
        }
        _ => {}
    }
    Ok(())
}

async fn callback_handler(bot: Bot, q: CallbackQuery, states: Arc<Mutex<HashMap<ChatId, UserState>>>, portfolios: PortfolioStore, watchlist: WatchlistStore) -> ResponseResult<()> {
    if let (Some(data), Some(msg)) = (q.data, q.message) {
        let chat_id = msg.chat().id;
        let mut state_lock = states.lock().await;
        match data.as_str() {
            "menu_crypto" => { state_lock.insert(chat_id, UserState::AwaitingCrypto); bot.send_message(chat_id, get_crypto_info_text()).parse_mode(ParseMode::Html).await?; }
            "menu_sharia" => { state_lock.insert(chat_id, UserState::AwaitingStock); bot.send_message(chat_id, get_stock_info_text()).parse_mode(ParseMode::Html).await?; }
            "menu_sentiment_info" => { bot.send_message(chat_id, get_sentiment_info_text()).parse_mode(ParseMode::Html).reply_markup(InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback("🎭 F&G INDEX", "menu_fng"), InlineKeyboardButton::callback("📈 PULSE", "menu_pulse")]])).await?; }
            "menu_fng" => { if let Ok(fng) = get_fear_and_greed().await { bot.send_message(chat_id, format!("🎭 <b>F&G INDEX: {} ({})</b>", fng.value, fng.value_classification)).parse_mode(ParseMode::Html).reply_markup(make_result_footer()).await?; } }
            "menu_pulse" => { state_lock.insert(chat_id, UserState::AwaitingSentiment); bot.send_message(chat_id, "📈 <b>Masukkan Ticker/Nama Koin:</b>\n(Contoh: BTC atau bitcoin)").await?; }
            "menu_buy" => { state_lock.insert(chat_id, UserState::AwaitingBuyTicker); bot.send_message(chat_id, get_trading_info_text()).parse_mode(ParseMode::Html).await?; }
            "menu_portfolio" => {
                let p = portfolios.lock().await.get(&chat_id).cloned().unwrap_or(UserPortfolio { balance: 10000.0, holdings: HashMap::new() });
                let mut res = format!("💼 <b>PORTFOLIO ENGINE</b>\n━━━━━━━━━━━━━━━━━━━━━\n💵 <b>Cash:</b> ${:.2}\n\nAssets:\n", p.balance);
                for h in p.holdings.values() { res.push_str(&format!("• {}: {:.4} (Avg ${:.2})\n", h.symbol, h.quantity, h.avg_price)); }
                bot.send_message(chat_id, res).parse_mode(ParseMode::Html).reply_markup(make_main_menu()).await?;
            }
            "menu_watchlist" => {
                let list = watchlist.lock().await.get(&chat_id).cloned().unwrap_or_default();
                bot.send_message(chat_id, format!("⭐ <b>WATCHLIST ANDA:</b>\n{:?}", list)).reply_markup(make_main_menu()).await?;
            }
            "menu_help" => { bot.send_message(chat_id, get_help_text()).parse_mode(ParseMode::Html).reply_markup(make_main_menu()).await?; }
            "back_to_main" => { state_lock.insert(chat_id, UserState::Idle); bot.send_message(chat_id, get_welcome_text(&q.from.first_name)).parse_mode(ParseMode::Html).reply_markup(make_main_menu()).await?; }
            _ => {}
        }
    }
    bot.answer_callback_query(q.id).await?;
    Ok(())
}