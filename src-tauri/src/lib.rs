mod commands;
mod crypto;
mod db;
mod models;
mod state;

use state::AppState;
use std::time::Duration;
use tauri::window::Color;
use tauri::{Emitter, Listener, Manager};

const LOCK_CHECK_INTERVAL: Duration = Duration::from_secs(2);
const LOCKED_SLEEP_INTERVAL: Duration = Duration::from_secs(30);
const FORCE_SHOW_TIMEOUT: Duration = Duration::from_secs(8);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 数据库路径：优先使用设置中指定的 storage_path，否则用默认
    let db_path = db::default_db_path();
    let db = db::DbState::new(db_path).expect("数据库初始化失败");
    let app_state = AppState::new(db);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::has_master_password,
            commands::set_master_password,
            commands::unlock,
            commands::lock,
            commands::heartbeat,
            commands::create_entry,
            commands::update_entry,
            commands::get_entries,
            commands::get_entry,
            commands::delete_entry,
            commands::restore_entry,
            commands::permanently_delete_entry,
            commands::empty_trash,
            commands::get_tags,
            commands::create_tag,
            commands::delete_tag,
            commands::get_category_stats,
            commands::get_tag_stats,
            commands::get_settings,
            commands::get_database_path,
            commands::update_setting,
            commands::export_data,
            commands::import_data,
            commands::get_otp_entries,
            commands::create_otp_entry,
            commands::delete_otp_entry,
            commands::update_otp_entry,
            commands::import_otp_entries,
            #[cfg(feature = "qr-scan")]
            commands::capture_qr_code,
            commands::read_file_text,
            #[cfg(feature = "qr-scan")]
            commands::decode_qr_image,
            commands::list_categories,
            commands::create_category,
            commands::update_category,
            commands::delete_category,
            commands::list_templates,
            commands::get_template,
            commands::create_template,
            commands::update_template,
            commands::delete_template,
        ])
        .setup(|app| {
            // 启动即铺深色背景（与锁屏页渐变一致），防止窗口/WebView 创建阶段白屏闪烁
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_background_color(Some(Color(0x1f, 0x29, 0x37, 0xff)));
                // 窗口以 visible:false 启动，锁屏界面渲染完成后由前端通知再显示，
                // 从原理上消除“窗口已显示但内容未就绪”的白屏窗口期
                let win = window.clone();
                let _ = app.listen("app-ready", move |_| {
                    let _ = win.show();
                });
                // 兜底：若前端事件丢失，定时后强制显示，避免窗口永不出现
                let win2 = window.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(FORCE_SHOW_TIMEOUT);
                    let _ = win2.show();
                });
            }
            // 自动锁屏定时器：已解锁时每 2 秒检查活动时间；已锁屏时休眠等待，解锁时被 notify 唤醒
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                let state = handle.state::<AppState>();
                let interval = if state.get_key().is_some() {
                    let settings = commands::get_settings(state.clone()).unwrap_or(models::Settings {
                        lock_timeout: 30,
                        theme: "light".into(),
                        storage_path: String::new(),
                    });
                    let elapsed = state::now_secs().saturating_sub(state.get_last_activity());
                    if elapsed >= settings.lock_timeout {
                        state.clear_key();
                        let _ = handle.emit("app-locked", ());
                    }
                    LOCK_CHECK_INTERVAL
                } else {
                    LOCKED_SLEEP_INTERVAL
                };
                let (mtx, cvar) = &state.lock_timer;
                let _guard = mtx.lock().unwrap();
                let _ = cvar.wait_timeout(_guard, interval);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
