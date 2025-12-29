use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode, BotCommand};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::Utc;

mod api;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct UserKey { chat_id: ChatId, user_id: UserId }
impl UserKey { fn new(chat_id: ChatId, user_id: UserId) -> Self { Self { chat_id, user_id } } }

#[derive(Clone, PartialEq)]
enum UserState { 
    Idle, AwaitingCrypto, AwaitingStock, AwaitingSolanaTicker, AwaitingRealBuyCA,
    AwaitingBuyTicker, AwaitingSellTicker, AwaitingAddWatchlist,
    AwaitingBroadcast, AwaitingBanUser, AwaitingUnbanUser, 
    AwaitingDirectMsg, AwaitingAddGroup, AwaitingRemoveGroup, AwaitingGiftPremium,
}

#[derive(Debug, Clone)]
struct Holding { symbol: String, quantity: f64, avg_price: f64 }
#[derive(Debug, Clone)]
struct UserPortfolio { balance: f64, holdings: HashMap<String, Holding> }

struct AppState {
    states: Mutex<HashMap<UserKey, UserState>>,
    portfolios: Mutex<HashMap<ChatId, UserPortfolio>>,
    watchlist: Mutex<HashMap<ChatId, Vec<String>>>,
    users: Mutex<HashSet<ChatId>>,
    banned: Mutex<HashSet<ChatId>>,
    premium_groups: Mutex<HashSet<ChatId>>,
    premium_users: Mutex<HashSet<ChatId>>,
}

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

fn is_admin(chat_id: i64) -> bool { chat_id == 6187914291 }

fn get_welcome_text(name: &str) -> String {
    format!(
        "👋 <b>Welcome, {}</b>\n\n\
        💠 <b>LUBIX TERMINAL v9.5</b>\n\
        <i>The Ultimate Financial Suite</i>\n\
        ------------------------\n\
        📊 <b>SYSTEM STATUS</b>\n\
        🟢 Engine: <code>ONLINE</code>\n\
        🟣 Solana: <code>SYNCED</code>\n\
        🟠 Data: <code>REAL-TIME</code>\n\n\
        👨‍💻 <b>DEV:</b> Iqbal (11 RPL IDN)\n\
        ------------------------\n\
        <i>Pilih menu di bawah:</i>", name)
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    println!("🚀 LubixBot v9.5 Starting...");
    
    let bot = Bot::from_env();
    let commands = vec![
        BotCommand::new("start", "🏠 Dashboard"),
        BotCommand::new("kripto", "🪙 Cek Crypto"),
        BotCommand::new("saham", "🕌 Saham Syariah"),
        BotCommand::new("solana", "⚡️ Solana DEX"),
        BotCommand::new("sim", "🎮 Trading Sim"),
        BotCommand::new("panel", "🔐 Admin"),
        BotCommand::new("help", "❓ Help"),
    ];
    let _ = bot.set_my_commands(commands).await;

    let app_state = Arc::new(AppState {
        states: Mutex::new(HashMap::new()),
        portfolios: Mutex::new(HashMap::new()),
        watchlist: Mutex::new(HashMap::new()),
        users: Mutex::new(HashSet::new()),
        banned: Mutex::new(HashSet::new()),
        premium_groups: Mutex::new(HashSet::new()),
        premium_users: Mutex::new(HashSet::new()),
    });

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler));
    
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![app_state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn message_handler(bot: Bot, msg: Message, state: Arc<AppState>) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    let text = msg.text().unwrap_or("");
    let user_name = msg.from().map(|u| u.first_name.as_str()).unwrap_or("User");
    let user_id = match msg.from() { Some(u) => u.id, None => return Ok(()) };
    let user_key = UserKey::new(chat_id, user_id);

    state.users.lock().await.insert(chat_id);

    match text {
        "/start" => {
            state.states.lock().await.insert(user_key, UserState::Idle);
            bot.send_message(chat_id, get_welcome_text(user_name))
                .parse_mode(ParseMode::Html).reply_markup(make_main_menu()).await?;
        }
        "/kripto" => {
            state.states.lock().await.insert(user_key, UserState::AwaitingCrypto);
            bot.send_message(chat_id, "🪙 <b>CRYPTO</b>\n\nMasukkan ticker:\n<i>Contoh: BTC, ETH, SOL</i>")
                .parse_mode(ParseMode::Html).await?;
        }
        "/saham" => {
            state.states.lock().await.insert(user_key, UserState::AwaitingStock);
            bot.send_message(chat_id, "🕌 <b>SAHAM SYARIAH</b>\n\nMasukkan kode:\n<i>Contoh: BBRI, TLKM, ADRO</i>")
                .parse_mode(ParseMode::Html).await?;
        }
        "/solana" => {
            state.states.lock().await.insert(user_key, UserState::AwaitingSolanaTicker);
            bot.send_message(chat_id, "⚡️ <b>SOLANA DEX</b>\n\nMasukkan ticker atau CA:")
                .parse_mode(ParseMode::Html).await?;
        }
        "/sim" => {
            bot.send_message(chat_id, "🎮 <b>TRADING SIMULATOR</b>\n\n💰 Starting: $10,000\n\nGunakan tombol di bawah:")
                .parse_mode(ParseMode::Html).reply_markup(make_sim_menu()).await?;
        }
        "/panel" => {
            if is_admin(chat_id.0) {
                let users_count = state.users.lock().await.len();
                let banned_count = state.banned.lock().await.len();
                let premium_count = state.premium_users.lock().await.len();
                let groups_count = state.premium_groups.lock().await.len();
                let active_portfolios = state.portfolios.lock().await.len();
                let watchlist_count = state.watchlist.lock().await.len();
                
                let current_time = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
                
                let admin_overview = format!(
                    "🛡️ <b>LUBIX ADMIN PANEL v9.5</b>\n\
                    ━━━━━━━━━━━━━━━━━━━━━━━\n\n\
                    📊 <b>SYSTEM OVERVIEW</b>\n\
                    ├ 🖥 Status: <code>🟢 ONLINE</code>\n\
                    ├ ⏰ Time: <code>{}</code>\n\
                    └ 🔄 Uptime: <code>Active</code>\n\n\
                    👥 <b>USER STATISTICS</b>\n\
                    ├ 📈 Total Users: <code>{}</code>\n\
                    ├ 🌟 Premium Users: <code>{}</code>\n\
                    ├ 🚫 Banned Users: <code>{}</code>\n\
                    └ 📊 Conversion: <code>{:.1}%</code>\n\n\
                    💼 <b>ENGAGEMENT DATA</b>\n\
                    ├ 📂 Active Portfolios: <code>{}</code>\n\
                    ├ ⭐ Watchlists: <code>{}</code>\n\
                    └ 👥 Premium Groups: <code>{}</code>\n\n\
                    🔧 <b>QUICK ACTIONS</b>\n\
                    Gunakan tombol di bawah untuk mengelola bot.\n\
                    ━━━━━━━━━━━━━━━━━━━━━━━",
                    current_time,
                    users_count,
                    premium_count,
                    banned_count,
                    if users_count > 0 { (premium_count as f64 / users_count as f64) * 100.0 } else { 0.0 },
                    active_portfolios,
                    watchlist_count,
                    groups_count
                );
                
                bot.send_message(chat_id, admin_overview)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(make_admin_menu()).await?;
            } else {
                bot.send_message(chat_id, "❌ <b>Access Denied</b>\n\nAnda tidak memiliki akses ke Admin Panel.").parse_mode(ParseMode::Html).await?;
            }
        }
        "/help" => {
            let help_text = format!(
                "📚 <b>LUBIX TERMINAL - HELP CENTER</b>\n\
                ━━━━━━━━━━━━━━━━━━━━━━━\n\n\
                🎯 <b>FITUR UTAMA</b>\n\
                ├ 🪙 /kripto - Cek harga cryptocurrency\n\
                ├ 🕌 /saham - Screening saham syariah\n\
                ├ ⚡️ /solana - Solana DEX tracker\n\
                ├ 🎮 /sim - Trading simulator\n\
                └ 🏠 /start - Kembali ke dashboard\n\n\
                📊 <b>MARKET TOOLS</b>\n\
                ├ 🌡 Sentiment - Fear & Greed Index\n\
                ├ ⭐ Watchlist - Pantau crypto favorit\n\
                ├ 💼 Portfolio - Lihat posisi trading\n\
                └ 🚀 Real Buy - Beli token Solana (soon)\n\n\
                💎 <b>PREMIUM FEATURES</b>\n\
                ├ ♾ Unlimited screening\n\
                ├ 📊 Advanced analytics\n\
                ├ 🔔 Price alerts\n\
                └ 💬 Priority support\n\n\
                ━━━━━━━━━━━━━━━━━━━━━━━\n\
                🌐 <b>CONNECT WITH US</b>\n\n\
                📸 <b>Instagram:</b>\n\
                └ <a href=\"https://instagram.com/herebou\">@herebou</a>\n\n\
                💼 <b>LinkedIn:</b>\n\
                └ <a href=\"https://linkedin.com/in/iqbaladiatma\">Iqbal Adiatma</a>\n\n\
                💻 <b>GitHub:</b>\n\
                └ <a href=\"https://github.com/herebou\">@herebou</a>\n\n\
                ━━━━━━━━━━━━━━━━━━━━━━━\n\
                📞 <b>CONTACT & SUPPORT</b>\n\n\
                Ada pertanyaan atau butuh bantuan?\n\
                📩 Hubungi: <a href=\"https://t.me/herebou\">@herebou</a>\n\n\
                💡 <i>Tips: Gunakan /start untuk kembali ke menu utama</i>\n\
                ━━━━━━━━━━━━━━━━━━━━━━━\n\
                👨‍💻 <b>Developer:</b> Iqbal (11 RPL IDN)\n\
                🔖 <b>Version:</b> v9.5"
            );
            bot.send_message(chat_id, help_text)
                .parse_mode(ParseMode::Html)
                .disable_web_page_preview(true)
                .reply_markup(make_help_menu()).await?;
        }
        _ => {
            let current_state = state.states.lock().await.get(&user_key).cloned().unwrap_or(UserState::Idle);
            match current_state {
                UserState::AwaitingCrypto => {
                    let result = api::fetch_crypto_from_cmc(text).await;
                    match result {
                        Ok(data) => { bot.send_message(chat_id, data).parse_mode(ParseMode::Html).reply_markup(make_back_menu()).await?; }
                        Err(e) => { bot.send_message(chat_id, format!("❌ {}", e)).await?; }
                    }
                    state.states.lock().await.insert(user_key, UserState::Idle);
                }
                UserState::AwaitingStock => {
                    let result = api::fetch_stock_from_api(text).await;
                    match result {
                        Ok(data) => { bot.send_message(chat_id, data).parse_mode(ParseMode::Html).reply_markup(make_back_menu()).await?; }
                        Err(e) => { bot.send_message(chat_id, format!("❌ {}", e)).await?; }
                    }
                    state.states.lock().await.insert(user_key, UserState::Idle);
                }
                UserState::AwaitingSolanaTicker => {
                    let result = api::fetch_solana_token(text).await;
                    match result {
                        Ok(data) => { bot.send_message(chat_id, data).parse_mode(ParseMode::Html).reply_markup(make_back_menu()).await?; }
                        Err(e) => { bot.send_message(chat_id, format!("❌ {}", e)).await?; }
                    }
                    state.states.lock().await.insert(user_key, UserState::Idle);
                }
                UserState::AwaitingRealBuyCA => {
                    bot.send_message(chat_id, format!("🚀 <b>REAL BUY</b>\n\nCA: <code>{}</code>\n\n⚠️ Feature dalam pengembangan!", text))
                        .parse_mode(ParseMode::Html).await?;
                    state.states.lock().await.insert(user_key, UserState::Idle);
                }
                UserState::AwaitingBuyTicker => {
                    let ticker = text.trim().to_uppercase();
                    match execute_buy(&state, chat_id, &ticker).await {
                        Ok(msg) => { bot.send_message(chat_id, msg).parse_mode(ParseMode::Html).reply_markup(make_sim_menu()).await?; }
                        Err(e) => { bot.send_message(chat_id, format!("❌ {}", e)).reply_markup(make_sim_menu()).await?; }
                    }
                    state.states.lock().await.insert(user_key, UserState::Idle);
                }
                UserState::AwaitingSellTicker => {
                    let ticker = text.trim().to_uppercase();
                    match execute_sell(&state, chat_id, &ticker).await {
                        Ok(msg) => { bot.send_message(chat_id, msg).parse_mode(ParseMode::Html).reply_markup(make_sim_menu()).await?; }
                        Err(e) => { bot.send_message(chat_id, format!("❌ {}", e)).reply_markup(make_sim_menu()).await?; }
                    }
                    state.states.lock().await.insert(user_key, UserState::Idle);
                }
                UserState::AwaitingAddWatchlist => {
                    let ticker = text.trim().to_uppercase();
                    state.watchlist.lock().await.entry(chat_id).or_insert_with(Vec::new).push(ticker.clone());
                    bot.send_message(chat_id, format!("✅ {} ditambahkan ke watchlist!", ticker)).reply_markup(make_watchlist_menu()).await?;
                    state.states.lock().await.insert(user_key, UserState::Idle);
                }
                UserState::AwaitingBroadcast => {
                    if is_admin(chat_id.0) {
                        let users = state.users.lock().await.clone();
                        let mut sent = 0;
                        for uid in users {
                            if let Ok(_) = bot.send_message(uid, format!("📢 <b>BROADCAST</b>\n\n{}", text)).parse_mode(ParseMode::Html).await {
                                sent += 1;
                            }
                        }
                        bot.send_message(chat_id, format!("✅ Broadcast sent to {} users", sent)).reply_markup(make_admin_action_menu()).await?;
                    }
                    state.states.lock().await.insert(user_key, UserState::Idle);
                }
                UserState::AwaitingBanUser => {
                    if is_admin(chat_id.0) {
                        if let Ok(uid) = text.trim().parse::<i64>() {
                            state.banned.lock().await.insert(ChatId(uid));
                            bot.send_message(chat_id, format!("🚫 User {} banned!", uid)).reply_markup(make_admin_action_menu()).await?;
                        }
                    }
                    state.states.lock().await.insert(user_key, UserState::Idle);
                }
                UserState::AwaitingUnbanUser => {
                    if is_admin(chat_id.0) {
                        if let Ok(uid) = text.trim().parse::<i64>() {
                            state.banned.lock().await.remove(&ChatId(uid));
                            bot.send_message(chat_id, format!("✅ User {} unbanned!", uid)).reply_markup(make_admin_action_menu()).await?;
                        }
                    }
                    state.states.lock().await.insert(user_key, UserState::Idle);
                }
                UserState::AwaitingGiftPremium => {
                    if is_admin(chat_id.0) {
                        if let Ok(uid) = text.trim().parse::<i64>() {
                            state.premium_users.lock().await.insert(ChatId(uid));
                            bot.send_message(chat_id, format!("🎁 Premium granted to {}!", uid)).reply_markup(make_admin_action_menu()).await?;
                        }
                    }
                    state.states.lock().await.insert(user_key, UserState::Idle);
                }
                UserState::AwaitingDirectMsg => {
                    if is_admin(chat_id.0) {
                        let parts: Vec<&str> = text.splitn(2, '|').collect();
                        if parts.len() == 2 {
                            if let Ok(uid) = parts[0].trim().parse::<i64>() {
                                let _ = bot.send_message(ChatId(uid), format!("📩 <b>Message from Admin:</b>\n\n{}", parts[1])).parse_mode(ParseMode::Html).await;
                                bot.send_message(chat_id, "✅ Message sent!").reply_markup(make_admin_action_menu()).await?;
                            }
                        }
                    }
                    state.states.lock().await.insert(user_key, UserState::Idle);
                }
                UserState::AwaitingAddGroup => {
                    if is_admin(chat_id.0) {
                        if let Ok(gid) = text.trim().parse::<i64>() {
                            state.premium_groups.lock().await.insert(ChatId(gid));
                            bot.send_message(chat_id, format!("✅ Group {} added!", gid)).reply_markup(make_admin_action_menu()).await?;
                        }
                    }
                    state.states.lock().await.insert(user_key, UserState::Idle);
                }
                UserState::AwaitingRemoveGroup => {
                    if is_admin(chat_id.0) {
                        if let Ok(gid) = text.trim().parse::<i64>() {
                            state.premium_groups.lock().await.remove(&ChatId(gid));
                            bot.send_message(chat_id, format!("✅ Group {} removed!", gid)).reply_markup(make_admin_action_menu()).await?;
                        }
                    }
                    state.states.lock().await.insert(user_key, UserState::Idle);
                }
                _ => { bot.send_message(chat_id, "Gunakan /start untuk memulai").await?; }
            }
        }
    }
    Ok(())
}

async fn callback_handler(bot: Bot, q: CallbackQuery, state: Arc<AppState>) -> ResponseResult<()> {
    let chat_id = q.message.as_ref().unwrap().chat.id;
    let data = q.data.as_deref().unwrap_or("");
    let user_name = q.from.first_name.as_str();
    let user_id = q.from.id;
    let user_key = UserKey::new(chat_id, user_id);

    match data {
        "back_to_main" => {
            bot.send_message(chat_id, get_welcome_text(user_name)).parse_mode(ParseMode::Html).reply_markup(make_main_menu()).await?;
        }
        "back_to_panel" => {
            if is_admin(chat_id.0) {
                let users_count = state.users.lock().await.len();
                let banned_count = state.banned.lock().await.len();
                let premium_count = state.premium_users.lock().await.len();
                let groups_count = state.premium_groups.lock().await.len();
                let active_portfolios = state.portfolios.lock().await.len();
                let watchlist_count = state.watchlist.lock().await.len();
                let current_time = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
                let admin_overview = format!(
                    "🛡️ <b>LUBIX ADMIN PANEL v9.5</b>\n━━━━━━━━━━━━━━━━━━━━━━━\n\n📊 <b>SYSTEM OVERVIEW</b>\n├ 🖥 Status: <code>🟢 ONLINE</code>\n├ ⏰ Time: <code>{}</code>\n└ 🔄 Uptime: <code>Active</code>\n\n👥 <b>USER STATISTICS</b>\n├ 📈 Total Users: <code>{}</code>\n├ 🌟 Premium Users: <code>{}</code>\n├ 🚫 Banned Users: <code>{}</code>\n└ 📊 Conversion: <code>{:.1}%</code>\n\n💼 <b>ENGAGEMENT DATA</b>\n├ 📂 Active Portfolios: <code>{}</code>\n├ ⭐ Watchlists: <code>{}</code>\n└ 👥 Premium Groups: <code>{}</code>\n\n🔧 <b>QUICK ACTIONS</b>\nGunakan tombol di bawah untuk mengelola bot.\n━━━━━━━━━━━━━━━━━━━━━━━",
                    current_time, users_count, premium_count, banned_count,
                    if users_count > 0 { (premium_count as f64 / users_count as f64) * 100.0 } else { 0.0 },
                    active_portfolios, watchlist_count, groups_count
                );
                bot.send_message(chat_id, admin_overview).parse_mode(ParseMode::Html).reply_markup(make_admin_menu()).await?;
            } else {
                bot.send_message(chat_id, "❌ Access Denied").await?;
            }
        }
        "menu_crypto" => {
            state.states.lock().await.insert(user_key, UserState::AwaitingCrypto);
            bot.send_message(chat_id, "🪙 <b>CRYPTO</b>\n\nMasukkan ticker:\n<i>Contoh: BTC, ETH, SOL</i>").parse_mode(ParseMode::Html).await?;
        }
        "menu_sharia" => {
            state.states.lock().await.insert(user_key, UserState::AwaitingStock);
            bot.send_message(chat_id, "🕌 <b>SAHAM SYARIAH</b>\n\nMasukkan kode:\n<i>Contoh: BBRI, TLKM, ADRO</i>").parse_mode(ParseMode::Html).await?;
        }
        "menu_solana" => {
            state.states.lock().await.insert(user_key, UserState::AwaitingSolanaTicker);
            bot.send_message(chat_id, "⚡️ <b>SOLANA DEX</b>\n\nMasukkan ticker/CA:").parse_mode(ParseMode::Html).await?;
        }
        "menu_sim_main" => {
            bot.send_message(chat_id, "🎮 <b>TRADING SIMULATOR</b>\n\n💰 Starting: $10,000").parse_mode(ParseMode::Html).reply_markup(make_sim_menu()).await?;
        }
        "menu_sentiment_info" => {
            let fng = api::fetch_fear_greed_index().await.unwrap_or_else(|e| format!("❌ {}", e));
            bot.send_message(chat_id, fng).parse_mode(ParseMode::Html).reply_markup(make_sentiment_menu()).await?;
        }
        "sentiment_fng" => {
            let fng = api::fetch_fear_greed_index().await.unwrap_or_else(|e| format!("❌ {}", e));
            bot.send_message(chat_id, fng).parse_mode(ParseMode::Html).reply_markup(make_sentiment_menu()).await?;
        }
        "sentiment_pulse" => {
            let pulse = api::fetch_market_pulse().await.unwrap_or_else(|e| format!("❌ {}", e));
            bot.send_message(chat_id, pulse).parse_mode(ParseMode::Html).reply_markup(make_sentiment_menu()).await?;
        }
        "menu_watchlist" => {
            let wl = state.watchlist.lock().await;
            let items = wl.get(&chat_id).cloned().unwrap_or_default();
            let txt = if items.is_empty() { 
                "⭐ <b>WATCHLIST</b>\n\n📭 Kosong".to_string() 
            } else {
                format!("⭐ <b>WATCHLIST</b>\n\n{}", items.iter().enumerate().map(|(i,s)| format!("{}. <code>{}</code>", i+1, s)).collect::<Vec<_>>().join("\n"))
            };
            bot.send_message(chat_id, txt).parse_mode(ParseMode::Html).reply_markup(make_watchlist_menu()).await?;
        }
        "watchlist_add" => {
            state.states.lock().await.insert(user_key, UserState::AwaitingAddWatchlist);
            bot.send_message(chat_id, "⭐ Masukkan ticker:\n<i>Contoh: BTC, ETH</i>").parse_mode(ParseMode::Html).await?;
        }
        "watchlist_clear" => {
            state.watchlist.lock().await.remove(&chat_id);
            bot.send_message(chat_id, "✅ Watchlist cleared!").reply_markup(make_main_menu()).await?;
        }
        "watchlist_check" => {
            let wl = state.watchlist.lock().await.get(&chat_id).cloned().unwrap_or_default();
            if wl.is_empty() { 
                bot.send_message(chat_id, "📭 Watchlist kosong!").await?; 
            } else {
                let mut res = "📊 <b>WATCHLIST PRICES</b>\n\n".to_string();
                for t in &wl {
                    match api::fetch_crypto_from_cmc(t).await {
                        Ok(d) => res.push_str(&format!("{}\n\n", d)),
                        Err(_) => res.push_str(&format!("❌ {} - Error\n", t)),
                    }
                }
                bot.send_message(chat_id, res).parse_mode(ParseMode::Html).reply_markup(make_watchlist_menu()).await?;
            }
        }
        "menu_solana_real" => {
            state.states.lock().await.insert(user_key, UserState::AwaitingRealBuyCA);
            bot.send_message(chat_id, "🚀 <b>REAL BUY SOLANA</b>\n\n⚠️ Transaksi NYATA!\n\nMasukkan CA token:").parse_mode(ParseMode::Html).await?;
        }
        "menu_help" => {
            let help_text = format!(
                "📚 <b>LUBIX TERMINAL - HELP CENTER</b>\n\
                ━━━━━━━━━━━━━━━━━━━━━━━\n\n\
                🎯 <b>FITUR UTAMA</b>\n\
                ├ 🪙 /kripto - Cek harga cryptocurrency\n\
                ├ 🕌 /saham - Screening saham syariah\n\
                ├ ⚡️ /solana - Solana DEX tracker\n\
                ├ 🎮 /sim - Trading simulator\n\
                └ 🏠 /start - Kembali ke dashboard\n\n\
                📊 <b>MARKET TOOLS</b>\n\
                ├ 🌡 Sentiment - Fear & Greed Index\n\
                ├ ⭐ Watchlist - Pantau crypto favorit\n\
                ├ 💼 Portfolio - Lihat posisi trading\n\
                └ 🚀 Real Buy - Beli token Solana (soon)\n\n\
                💎 <b>PREMIUM FEATURES</b>\n\
                ├ ♾ Unlimited screening\n\
                ├ 📊 Advanced analytics\n\
                ├ 🔔 Price alerts\n\
                └ 💬 Priority support\n\n\
                ━━━━━━━━━━━━━━━━━━━━━━━\n\
                🌐 <b>CONNECT WITH US</b>\n\n\
                📸 <b>Instagram:</b>\n\
                └ <a href=\"https://instagram.com/herebou\">@herebou</a>\n\n\
                💼 <b>LinkedIn:</b>\n\
                └ <a href=\"https://linkedin.com/in/iqbaladiatma\">Iqbal Adiatma</a>\n\n\
                💻 <b>GitHub:</b>\n\
                └ <a href=\"https://github.com/herebou\">@herebou</a>\n\n\
                ━━━━━━━━━━━━━━━━━━━━━━━\n\
                📞 <b>CONTACT & SUPPORT</b>\n\n\
                Ada pertanyaan atau butuh bantuan?\n\
                📩 Hubungi: <a href=\"https://t.me/herebou\">@herebou</a>\n\n\
                💡 <i>Tips: Gunakan /start untuk kembali ke menu utama</i>\n\
                ━━━━━━━━━━━━━━━━━━━━━━━\n\
                👨‍💻 <b>Developer:</b> Iqbal (11 RPL IDN)\n\
                🔖 <b>Version:</b> v9.5"
            );
            bot.send_message(chat_id, help_text)
                .parse_mode(ParseMode::Html)
                .disable_web_page_preview(true)
                .reply_markup(make_help_menu()).await?;
        }
        "menu_buy" => {
            state.states.lock().await.insert(user_key, UserState::AwaitingBuyTicker);
            bot.send_message(chat_id, "📈 <b>BUY $1000</b>\n\nMasukkan ticker:\n<i>BTC, ETH, SOL, BNB, ADA</i>").parse_mode(ParseMode::Html).await?;
        }
        "menu_sell" => {
            state.states.lock().await.insert(user_key, UserState::AwaitingSellTicker);
            bot.send_message(chat_id, "📉 <b>SELL ALL</b>\n\nMasukkan ticker yang ingin dijual:").parse_mode(ParseMode::Html).await?;
        }
        "menu_portfolio" => {
            let portfolio = get_portfolio(&state, chat_id).await;
            let mut total = portfolio.balance;
            let mut holdings_txt = String::new();
            for (sym, h) in &portfolio.holdings {
                let price = get_price(sym).await;
                let val = h.quantity * price;
                total += val;
                let pnl = val - (h.quantity * h.avg_price);
                let emoji = if pnl >= 0.0 { "📈" } else { "📉" };
                holdings_txt.push_str(&format!("• <b>{}</b>: {:.4} @ ${:.2} = ${:.2} {} ${:.2}\n", sym, h.quantity, price, val, emoji, pnl.abs()));
            }
            if holdings_txt.is_empty() { holdings_txt = "<i>No positions</i>\n".to_string(); }
            let pnl_total = total - 10000.0;
            let emoji = if pnl_total >= 0.0 { "📈" } else { "📉" };
            bot.send_message(chat_id, format!("💼 <b>PORTFOLIO</b>\n\n💰 Cash: <code>${:.2}</code>\n📊 Total: <code>${:.2}</code>\n{} P&L: <code>${:.2}</code>\n\n<b>Holdings:</b>\n{}", portfolio.balance, total, emoji, pnl_total.abs(), holdings_txt)).parse_mode(ParseMode::Html).reply_markup(make_sim_menu()).await?;
        }
        "admin_dashboard" => {
            if is_admin(chat_id.0) {
                let users_count = state.users.lock().await.len();
                let banned_count = state.banned.lock().await.len();
                let premium_count = state.premium_users.lock().await.len();
                let groups_count = state.premium_groups.lock().await.len();
                let active_portfolios = state.portfolios.lock().await.len();
                let watchlist_count = state.watchlist.lock().await.len();
                
                let current_time = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
                
                let admin_overview = format!(
                    "🛡️ <b>LUBIX ADMIN PANEL v9.5</b>\n\
                    ━━━━━━━━━━━━━━━━━━━━━━━\n\n\
                    📊 <b>SYSTEM OVERVIEW</b>\n\
                    ├ 🖥 Status: <code>🟢 ONLINE</code>\n\
                    ├ ⏰ Time: <code>{}</code>\n\
                    └ � Uptime: <code>Active</code>\n\n\
                    �👥 <b>USER STATISTICS</b>\n\
                    ├ 📈 Total Users: <code>{}</code>\n\
                    ├ 🌟 Premium Users: <code>{}</code>\n\
                    ├ 🚫 Banned Users: <code>{}</code>\n\
                    └ 📊 Conversion: <code>{:.1}%</code>\n\n\
                    💼 <b>ENGAGEMENT DATA</b>\n\
                    ├ 📂 Active Portfolios: <code>{}</code>\n\
                    ├ ⭐ Watchlists: <code>{}</code>\n\
                    └ 👥 Premium Groups: <code>{}</code>\n\n\
                    🔧 <b>QUICK ACTIONS</b>\n\
                    Gunakan tombol di bawah untuk mengelola bot.\n\
                    ━━━━━━━━━━━━━━━━━━━━━━━",
                    current_time,
                    users_count,
                    premium_count,
                    banned_count,
                    if users_count > 0 { (premium_count as f64 / users_count as f64) * 100.0 } else { 0.0 },
                    active_portfolios,
                    watchlist_count,
                    groups_count
                );
                
                bot.send_message(chat_id, admin_overview)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(make_admin_menu()).await?;
            }
        }
        "admin_users" => {
            if is_admin(chat_id.0) {
                let users = state.users.lock().await;
                let list: Vec<String> = users.iter().take(20).enumerate().map(|(i, u)| format!("{}. <code>{}</code>", i+1, u.0)).collect();
                bot.send_message(chat_id, format!("👥 <b>USERS</b> ({})\n\n{}", users.len(), list.join("\n"))).parse_mode(ParseMode::Html).reply_markup(make_admin_action_menu()).await?;
            }
        }
        "admin_broadcast" => {
            if is_admin(chat_id.0) {
                state.states.lock().await.insert(user_key, UserState::AwaitingBroadcast);
                bot.send_message(chat_id, "📢 Ketik pesan broadcast:").await?;
            }
        }
        "admin_ban" => {
            if is_admin(chat_id.0) {
                state.states.lock().await.insert(user_key, UserState::AwaitingBanUser);
                bot.send_message(chat_id, "🚫 Masukkan User ID:").await?;
            }
        }
        "admin_unban" => {
            if is_admin(chat_id.0) {
                state.states.lock().await.insert(user_key, UserState::AwaitingUnbanUser);
                bot.send_message(chat_id, "✅ Masukkan User ID:").await?;
            }
        }
        "admin_gift_premium" => {
            if is_admin(chat_id.0) {
                state.states.lock().await.insert(user_key, UserState::AwaitingGiftPremium);
                bot.send_message(chat_id, "🎁 Masukkan User ID:").await?;
            }
        }
        "admin_dm" => {
            if is_admin(chat_id.0) {
                state.states.lock().await.insert(user_key, UserState::AwaitingDirectMsg);
                bot.send_message(chat_id, "📩 Format: USER_ID|Pesan").await?;
            }
        }
        "admin_add_group" => {
            if is_admin(chat_id.0) {
                state.states.lock().await.insert(user_key, UserState::AwaitingAddGroup);
                bot.send_message(chat_id, "➕ Masukkan Group ID:").await?;
            }
        }
        "admin_rem_group" => {
            if is_admin(chat_id.0) {
                state.states.lock().await.insert(user_key, UserState::AwaitingRemoveGroup);
                bot.send_message(chat_id, "➖ Masukkan Group ID:").await?;
            }
        }
        "admin_wallet" | "admin_analytics" | "admin_system" => {
            if is_admin(chat_id.0) {
                bot.send_message(chat_id, "🔧 <b>SYSTEM STATUS</b>\n\n🟢 All systems operational").parse_mode(ParseMode::Html).reply_markup(make_admin_action_menu()).await?;
            }
        }
        _ => { bot.send_message(chat_id, "✅ Feature aktif!").await?; }
    }
    bot.answer_callback_query(q.id).await?;
    Ok(())
}

async fn get_portfolio(state: &Arc<AppState>, chat_id: ChatId) -> UserPortfolio {
    state.portfolios.lock().await.entry(chat_id).or_insert_with(|| UserPortfolio { balance: 10000.0, holdings: HashMap::new() }).clone()
}

async fn get_price(symbol: &str) -> f64 {
    api::get_real_crypto_price(symbol).await.unwrap_or_else(|_| {
        match symbol { "BTC" => 94000.0, "ETH" => 3400.0, "SOL" => 190.0, "BNB" => 700.0, "ADA" => 0.9, _ => 100.0 }
    })
}

async fn execute_buy(state: &Arc<AppState>, chat_id: ChatId, symbol: &str) -> Result<String, String> {
    let price = get_price(symbol).await;
    let amount = 1000.0;
    let qty = amount / price;
    let mut portfolios = state.portfolios.lock().await;
    let p = portfolios.entry(chat_id).or_insert_with(|| UserPortfolio { balance: 10000.0, holdings: HashMap::new() });
    if p.balance < amount { return Err("Insufficient balance!".to_string()); }
    p.balance -= amount;
    let h = p.holdings.entry(symbol.to_string()).or_insert(Holding { symbol: symbol.to_string(), quantity: 0.0, avg_price: 0.0 });
    let total_cost = h.quantity * h.avg_price + amount;
    h.quantity += qty;
    h.avg_price = total_cost / h.quantity;
    Ok(format!("✅ <b>BUY ORDER</b>\n\n{}: {:.6} @ ${:.2}\nTotal: ${:.2}\nBalance: ${:.2}", symbol, qty, price, amount, p.balance))
}

async fn execute_sell(state: &Arc<AppState>, chat_id: ChatId, symbol: &str) -> Result<String, String> {
    let price = get_price(symbol).await;
    let mut portfolios = state.portfolios.lock().await;
    let p = portfolios.entry(chat_id).or_insert_with(|| UserPortfolio { balance: 10000.0, holdings: HashMap::new() });
    let h = p.holdings.remove(symbol).ok_or("No holdings for this crypto")?;
    let value = h.quantity * price;
    let pnl = value - (h.quantity * h.avg_price);
    p.balance += value;
    let emoji = if pnl >= 0.0 { "📈" } else { "📉" };
    Ok(format!("✅ <b>SELL ORDER</b>\n\n{}: {:.6} @ ${:.2}\nValue: ${:.2}\n{} P&L: ${:.2}\nBalance: ${:.2}", symbol, h.quantity, price, value, emoji, pnl.abs(), p.balance))
}

fn make_main_menu() -> InlineKeyboardMarkup { 
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🪙 CRYPTO", "menu_crypto"), InlineKeyboardButton::callback("🕌 SAHAM", "menu_sharia")],
        vec![InlineKeyboardButton::callback("⚡️ SOLANA", "menu_solana"), InlineKeyboardButton::callback("🌡 SENTIMENT", "menu_sentiment_info")],
        vec![InlineKeyboardButton::callback("🎮 SIMULATOR", "menu_sim_main")],
        vec![InlineKeyboardButton::callback("🚀 REAL BUY", "menu_solana_real"), InlineKeyboardButton::callback("⭐ WATCHLIST", "menu_watchlist")],
        vec![InlineKeyboardButton::callback("❓ HELP", "menu_help"), InlineKeyboardButton::callback("🔄 REFRESH", "back_to_main")]
    ])
}

fn make_sim_menu() -> InlineKeyboardMarkup { 
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("📈 BUY $1000", "menu_buy"), InlineKeyboardButton::callback("📉 SELL ALL", "menu_sell")],
        vec![InlineKeyboardButton::callback("💼 PORTFOLIO", "menu_portfolio")],
        vec![InlineKeyboardButton::callback("🔙 BACK", "back_to_main")]
    ])
}

fn make_admin_menu() -> InlineKeyboardMarkup { 
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("📊 DASHBOARD", "admin_dashboard"), InlineKeyboardButton::callback("👥 USERS", "admin_users")],
        vec![InlineKeyboardButton::callback("💰 WALLET", "admin_wallet"), InlineKeyboardButton::callback("📈 ANALYTICS", "admin_analytics")],
        vec![InlineKeyboardButton::callback("🎁 GIFT PREMIUM", "admin_gift_premium"), InlineKeyboardButton::callback("📩 DM USER", "admin_dm")],
        vec![InlineKeyboardButton::callback("🚫 BAN", "admin_ban"), InlineKeyboardButton::callback("✅ UNBAN", "admin_unban")],
        vec![InlineKeyboardButton::callback("📢 BROADCAST", "admin_broadcast"), InlineKeyboardButton::callback("🔧 SYSTEM", "admin_system")],
        vec![InlineKeyboardButton::callback("➕ ADD GROUP", "admin_add_group"), InlineKeyboardButton::callback("➖ REM GROUP", "admin_rem_group")],
        vec![InlineKeyboardButton::callback("❌ CLOSE", "back_to_main")]
    ])
}

fn make_sentiment_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🎭 Fear & Greed", "sentiment_fng"), InlineKeyboardButton::callback("📈 Market Pulse", "sentiment_pulse")],
        vec![InlineKeyboardButton::callback("🔙 BACK", "back_to_main")]
    ])
}

fn make_watchlist_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("➕ ADD", "watchlist_add"), InlineKeyboardButton::callback("📊 CHECK ALL", "watchlist_check")],
        vec![InlineKeyboardButton::callback("🗑 CLEAR", "watchlist_clear"), InlineKeyboardButton::callback("🔙 BACK", "back_to_main")]
    ])
}

fn make_back_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🏠 HOME", "back_to_main"), InlineKeyboardButton::callback("🔄 REFRESH", "back_to_main")]
    ])
}

fn make_help_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::url("📸 Instagram", "https://instagram.com/herebou".parse().unwrap()),
            InlineKeyboardButton::url("💼 LinkedIn", "https://linkedin.com/in/iqbaladiatma".parse().unwrap())
        ],
        vec![
            InlineKeyboardButton::url("💻 GitHub", "https://github.com/herebou".parse().unwrap()),
            InlineKeyboardButton::url("📩 Contact", "https://t.me/herebou".parse().unwrap())
        ],
        vec![InlineKeyboardButton::callback("🏠 BACK TO HOME", "back_to_main")]
    ])
}

fn make_admin_action_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("🔄 REFRESH", "admin_dashboard"),
            InlineKeyboardButton::callback("🔙 PANEL", "back_to_panel")
        ],
        vec![InlineKeyboardButton::callback("❌ CLOSE", "back_to_main")]
    ])
}