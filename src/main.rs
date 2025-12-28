use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode, BotCommand};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::env;

// --- STRUKTUR DATA (LENGKAP) ---
#[derive(Clone, PartialEq)]
enum UserState { Idle, AwaitingCrypto, AwaitingStock, AwaitingSentiment, AwaitingAddWatchlist }

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
    #[serde(rename = "code")] kode: String, 
    #[serde(rename = "name")] nama: String, 
    sector: String, ipo: String, papan: String,
    #[serde(rename = "marketCap")] market_cap: f64,
    issi: bool,
    #[serde(rename = "syariahIndicator")] syariah_indicator: SyariahIndicator,
    harga: StockPrice,
}

#[derive(Deserialize, Debug, Clone)]
struct CmcResponse { data: HashMap<String, CryptoData> }

#[derive(Deserialize, Debug, Clone)]
struct CryptoData {
    name: String, symbol: String,
    quote: HashMap<String, QuoteData>,
}

#[derive(Deserialize, Debug, Clone)]
struct QuoteData { price: f64 }

// --- DATA SENTIMENT ENGINE ---
#[derive(Deserialize, Debug, Clone)]
struct AltTickerResponse { data: HashMap<String, AltTickerData> }

#[derive(Deserialize, Debug, Clone)]
struct AltTickerData {
    name: String, symbol: String,
    quotes: HashMap<String, AltQuote>,
}

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

type WatchlistStore = Arc<Mutex<HashMap<ChatId, Vec<String>>>>;

// --- FUNGSI SENTRAL TEKS ---

fn get_welcome_text(name: &str) -> String {
    format!(
        "<b>💎 LUBIX TERMINAL v2.3</b> 💎\n\
        <i>The Complete Financial Suite</i>\n\
        ━━━━━━━━━━━━━━━━━━━━━\n\n\
        Halo, <b>{}</b>. Selamat datang kembali di Terminal.\n\n\
        🚀 <b>LUBIX CORE STATUS:</b>\n\
        • 🛰 <b>Network:</b> <code>STABLE</code>\n\
        • 🧠 <b>Engine:</b> <code>V2.3 (All Features Restored)</code>\n\n\
        👨‍💻 <b>DEVELOPER:</b> Iqbal (11 RPL IDN Solo) — <b>Misi NUS 2026</b>\n\
        ━━━━━━━━━━━━━━━━━━━━━\n\
        <i>Silakan pilih modul di bawah untuk mulai:</i>", name
    )
}

fn get_help_text() -> String {
    "❓ <b>LUBIX HELP & DOCUMENTATION</b>\n\
    ━━━━━━━━━━━━━━━━━━━━━\n\n\
    📖 <b>DAFTAR PERINTAH:</b>\n\
    • /start - Reset & Dashboard\n\
    • /kripto - Modul Harga Crypto\n\
    • /saham - Modul Saham Syariah\n\
    • /sentiment - Analisis Psikologi Market\n\
    • /help - Pusat Bantuan\n\n\
    📬 <b>KONTAK:</b> Hubungi @herebou\n\
    ━━━━━━━━━━━━━━━━━━━━━".to_string()
}

// --- API FUNCTIONS ---

async fn get_market_pulse(symbol: &str) -> Result<AltTickerData, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://api.alternative.me/v2/ticker/{}/", symbol.to_lowercase());
    let client = reqwest::Client::new();
    let resp = client.get(url).send().await?.json::<AltTickerResponse>().await?;
    let data = resp.data.values().next().ok_or("Not Found")?.clone();
    Ok(data)
}

async fn get_fear_and_greed() -> Result<FngData, Box<dyn std::error::Error + Send + Sync>> {
    let url = "https://api.alternative.me/fng/";
    let client = reqwest::Client::new();
    let resp = client.get(url).send().await?.json::<FngResponse>().await?;
    Ok(resp.data[0].clone())
}

async fn get_sharia_detail(ticker: &str) -> Result<EmitenDetail, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://syariahsaham-api.fly.dev/emiten/{}", ticker.to_uppercase());
    let client = reqwest::Client::new();
    let resp = client.get(url).send().await?.json::<EmitenDetail>().await?;
    Ok(resp)
}

async fn get_crypto_price(symbol: &str) -> Result<(CryptoData, f64), Box<dyn std::error::Error + Send + Sync>> {
    let api_key = env::var("CMC_API_KEY").unwrap_or_default();
    let url = format!("https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?symbol={}&convert=USD", symbol.to_uppercase());
    let client = reqwest::Client::new();
    let resp = client.get(url).header("X-CMC_PRO_API_KEY", api_key).header("Accept", "application/json").send().await?.json::<CmcResponse>().await?;
    let data = resp.data.get(&symbol.to_uppercase()).ok_or("Not Found")?.clone();
    let usd = data.quote.get("USD").map(|q| q.price).unwrap_or(0.0);
    Ok((data, usd * 16000.0))
}

fn format_idr(n: f64) -> String {
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

// --- KEYBOARDS ---

fn make_main_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("💰 CRYPTO", "menu_crypto"), InlineKeyboardButton::callback("🕌 SAHAM", "menu_sharia")],
        vec![InlineKeyboardButton::callback("🌡 SENTIMENT", "menu_fng"), InlineKeyboardButton::callback("⭐ WATCHLIST", "menu_watchlist")],
        vec![InlineKeyboardButton::callback("❓ HELP CENTER", "menu_help")],
        vec![
            InlineKeyboardButton::url("👨‍💻 GITHUB", "https://github.com/usernamemu".parse().unwrap()),
            InlineKeyboardButton::url("📸 INSTAGRAM", "https://instagram.com/usernamemu".parse().unwrap()),
        ],
    ])
}

fn make_result_footer() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("🏠 HOME", "back_to_main"),
            InlineKeyboardButton::callback("🌡 CEK PULSE", "menu_pulse"),
        ]
    ])
}

// --- MAIN RUNTIME ---

fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all()
        .thread_stack_size(4 * 1024 * 1024).build().unwrap();

    runtime.block_on(async {
        println!("LubixBot v2.3 Restoration Edition ONLINE 🚀");
        let bot = Bot::from_env();
        
        let commands = vec![
            BotCommand::new("start", "🏠 Dashboard Utama"),
            BotCommand::new("kripto", "🪙 Market Crypto"),
            BotCommand::new("saham", "🕌 Sharia Stock"),
            BotCommand::new("sentiment", "🌡 Analisis Sentimen"),
            BotCommand::new("help", "❓ Bantuan"),
        ];
        let _ = bot.set_my_commands(commands).await;

        let user_states = Arc::new(Mutex::new(HashMap::<ChatId, UserState>::new()));
        let watchlist = Arc::new(Mutex::new(HashMap::<ChatId, Vec<String>>::new()));

        let handler = dptree::entry().branch(Update::filter_message().endpoint(message_handler)).branch(Update::filter_callback_query().endpoint(callback_handler));
        Dispatcher::builder(bot, handler).dependencies(dptree::deps![user_states, watchlist]).enable_ctrlc_handler().build().dispatch().await;
    });
}

// --- HANDLERS ---

async fn message_handler(bot: Bot, msg: Message, states: Arc<Mutex<HashMap<ChatId, UserState>>>, watchlist: WatchlistStore) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    let text = msg.text().unwrap_or("");
    let mut state_lock = states.lock().await;

    // Command Handling
    match text {
        "/start" => {
            state_lock.insert(chat_id, UserState::Idle);
            let name = msg.from.as_ref().map(|u| u.first_name.clone()).unwrap_or_else(|| "Trader".to_string());
            bot.send_message(chat_id, get_welcome_text(&name)).parse_mode(ParseMode::Html).reply_markup(make_main_menu()).await?;
            return Ok(());
        }
        "/help" => {
            bot.send_message(chat_id, get_help_text()).parse_mode(ParseMode::Html).reply_markup(make_result_footer()).await?;
            return Ok(());
        }
        "/sentiment" => {
            bot.send_message(chat_id, "🌡 <b>MODUL SENTIMENT</b>\nAnalisis psikologi market global.").parse_mode(ParseMode::Html).reply_markup(make_main_menu()).await?;
            return Ok(());
        }
        _ => {}
    }

    let current_state = state_lock.get(&chat_id).cloned().unwrap_or(UserState::Idle);
    match (current_state, text) {
        (UserState::AwaitingSentiment, ticker) if !ticker.starts_with('/') => {
            bot.send_message(chat_id, format!("🌡 <b>Analysing Pulse:</b> <code>{}</code>...", ticker.to_uppercase())).parse_mode(ParseMode::Html).await?;
            if let Ok(d) = get_market_pulse(ticker).await {
                let q = d.quotes.get("USD").unwrap();
                let res = format!("📈 <b>MARKET PULSE: {}</b>\n━━━━━━━━━━━━━━━━━━━━━\n🕐 1H: <code>{:.2}%</code>\n📅 24H: <code>{:.2}%</code>\n🗓 7D: <code>{:.2}%</code>\n━━━━━━━━━━━━━━━━━━━━━", d.name, q.change_1h, q.change_24h, q.change_7d);
                bot.send_message(chat_id, res).parse_mode(ParseMode::Html).reply_markup(make_result_footer()).await?;
            } else { bot.send_message(chat_id, "❌ Data Pulse tidak ditemukan.").await?; }
            state_lock.insert(chat_id, UserState::Idle);
        }
        (UserState::AwaitingStock, ticker) if !ticker.starts_with('/') => {
            if let Ok(e) = get_sharia_detail(ticker).await {
                let res = format!("🏢 <b>{}</b>\n💰 Price: Rp {}\n🕌 Status: {}", e.nama.to_uppercase(), format_idr(e.harga.now as f64), if e.issi { "✅ Syariah" } else { "❌ Non-Syariah" });
                bot.send_message(chat_id, res).parse_mode(ParseMode::Html).reply_markup(make_result_footer()).await?;
            } else { bot.send_message(chat_id, "❌ Emiten tidak ditemukan.").await?; }
            state_lock.insert(chat_id, UserState::Idle);
        }
        (UserState::AwaitingCrypto, ticker) if !ticker.starts_with('/') => {
            if let Ok((data, idr)) = get_crypto_price(ticker).await {
                let res = format!("🪙 <b>{}</b>\n💵 USD: ${:.2}\n🇮🇩 IDR: Rp {}", data.name, data.quote.get("USD").unwrap().price, format_idr(idr));
                bot.send_message(chat_id, res).parse_mode(ParseMode::Html).reply_markup(make_result_footer()).await?;
            } else { bot.send_message(chat_id, "❌ Koin tidak ditemukan.").await?; }
            state_lock.insert(chat_id, UserState::Idle);
        }
        _ => {}
    }
    Ok(())
}

async fn callback_handler(bot: Bot, q: CallbackQuery, states: Arc<Mutex<HashMap<ChatId, UserState>>>, watchlist: WatchlistStore) -> ResponseResult<()> {
    if let (Some(data), Some(msg)) = (q.data, q.message) {
        let chat_id = msg.chat().id;
        let mut state_lock = states.lock().await;
        match data.as_str() {
            "menu_fng" => {
                if let Ok(fng) = get_fear_and_greed().await {
                    let res = format!("🎭 <b>MARKET SENTIMENT</b>\n━━━━━━━━━━━━━━━━━━━━━\nNilai: <b>{}</b>\nStatus: <b>{}</b>", fng.value, fng.value_classification);
                    bot.send_message(chat_id, res).parse_mode(ParseMode::Html).reply_markup(make_result_footer()).await?;
                }
            }
            "menu_pulse" => { state_lock.insert(chat_id, UserState::AwaitingSentiment); bot.send_message(chat_id, "📈 <b>Masukkan Nama Koin untuk Pulse:</b>\n(Contoh: bitcoin)").parse_mode(ParseMode::Html).await?; }
            "menu_watchlist" => {
                let list = watchlist.lock().await.get(&chat_id).cloned().unwrap_or_default();
                bot.send_message(chat_id, format!("⭐ <b>WATCHLIST:</b>\n{:?}", list)).reply_markup(make_result_footer()).await?;
            }
            "menu_crypto" => { state_lock.insert(chat_id, UserState::AwaitingCrypto); bot.send_message(chat_id, "💰 <b>Masukkan Ticker Crypto:</b>").await?; }
            "menu_sharia" => { state_lock.insert(chat_id, UserState::AwaitingStock); bot.send_message(chat_id, "🕌 <b>Masukkan Kode Saham:</b>").await?; }
            "menu_help" => { bot.send_message(chat_id, get_help_text()).parse_mode(ParseMode::Html).reply_markup(make_result_footer()).await?; }
            "back_to_main" => {
                state_lock.insert(chat_id, UserState::Idle);
                bot.send_message(chat_id, get_welcome_text(&q.from.first_name)).parse_mode(ParseMode::Html).reply_markup(make_main_menu()).await?;
            }
            _ => {}
        }
    }
    bot.answer_callback_query(q.id).await?;
    Ok(())
}