use super::{Result, ensure_db_unlocked};
use crate::models::*;
use crate::state::AppState;
use rusqlite::params;
#[cfg(feature = "qr-scan")]
use tauri::Manager;
use tauri::State;

#[tauri::command]
pub fn get_otp_entries(state: State<AppState>) -> Result<Vec<OtpEntryOut>> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, issuer, account, secret, interval, digits, algorithm, created_at FROM otp_entries ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let entries = stmt
        .query_map([], |r| {
            Ok(OtpEntryOut {
                id: r.get(0)?,
                issuer: r.get(1)?,
                account: r.get(2)?,
                secret: r.get(3)?,
                interval: r.get(4)?,
                digits: r.get(5)?,
                algorithm: r.get(6)?,
                created_at: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(entries)
}

#[tauri::command]
pub fn create_otp_entry(state: State<AppState>, input: OtpEntryInput) -> Result<OtpEntryOut> {
    ensure_db_unlocked(&state)?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::state::now_secs().to_string();
    let conn = state.db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO otp_entries (id, issuer, account, secret, interval, digits, algorithm, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![id, input.issuer, input.account, input.secret, input.interval, input.digits, input.algorithm, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(OtpEntryOut {
        id,
        issuer: input.issuer,
        account: input.account,
        secret: input.secret,
        interval: input.interval,
        digits: input.digits,
        algorithm: input.algorithm,
        created_at: now,
    })
}

#[tauri::command]
pub fn delete_otp_entry(state: State<AppState>, id: String) -> Result<()> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    conn.execute("DELETE FROM otp_entries WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_otp_entry(state: State<AppState>, id: String, input: OtpEntryInput) -> Result<OtpEntryOut> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    conn.execute(
        "UPDATE otp_entries SET issuer=?1, account=?2, secret=?3, interval=?4, digits=?5, algorithm=?6 WHERE id=?7",
        params![input.issuer, input.account, input.secret, input.interval, input.digits, input.algorithm, id],
    )
    .map_err(|e| e.to_string())?;
    let row: std::result::Result<(String, String, String, String, u32, u32, String, String), _> = conn.query_row(
        "SELECT id, issuer, account, secret, interval, digits, algorithm, created_at FROM otp_entries WHERE id=?1",
        params![id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
    );
    row.map(|r| OtpEntryOut {
        id: r.0,
        issuer: r.1,
        account: r.2,
        secret: r.3,
        interval: r.4,
        digits: r.5,
        algorithm: r.6,
        created_at: r.7,
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_otp_entries(state: State<AppState>, entries: Vec<OtpEntryInput>) -> Result<u32> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    let mut count = 0u32;
    for input in entries {
        let id = uuid::Uuid::new_v4().to_string();
        let now = crate::state::now_secs().to_string();
        conn.execute(
            "INSERT INTO otp_entries (id, issuer, account, secret, interval, digits, algorithm, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![id, input.issuer, input.account, input.secret, input.interval, input.digits, input.algorithm, now],
        )
        .map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

#[cfg(feature = "qr-scan")]
#[tauri::command]
pub async fn capture_qr_code(app: tauri::AppHandle) -> Result<String> {
    let window = app.get_webview_window("main").ok_or("窗口不存在")?;
    window.hide().map_err(|e| e.to_string())?;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let result = (|| -> Result<String> {
        let screenshot = screenshots::Screen::all()
            .map_err(|e| format!("截图失败: {}", e))?
            .into_iter()
            .next()
            .ok_or("未找到屏幕")?
            .capture()
            .map_err(|e| format!("截图失败: {}", e))?;
        let width = screenshot.width() as usize;
        let raw = screenshot.into_raw();
        let luma: Vec<u8> = raw.chunks(4).map(|p| {
            let r = p[0] as u32;
            let g = p[1] as u32;
            let b = p[2] as u32;
            ((r * 299 + g * 587 + b * 114) / 1000) as u8
        }).collect();
        let threshold = 128u8;
        let grid = rqrr::SimpleGrid::from_func(width, |x, y| {
            let idx = y * width + x;
            idx < luma.len() && luma[idx] < threshold
        });
        let decoded = rqrr::Grid::new(grid).decode().map_err(|_| "未检测到二维码")?;
        Ok(decoded.1)
    })();

    window.show().map_err(|e| e.to_string())?;
    result
}

#[tauri::command]
pub fn read_file_text(path: String) -> Result<String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("读取文件失败: {}", e))?;
    let content = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&bytes[3..]).to_string()
    } else {
        String::from_utf8_lossy(&bytes).to_string()
    };
    Ok(content)
}

#[cfg(feature = "qr-scan")]
#[tauri::command]
pub fn decode_qr_image(path: String) -> Result<String> {
    let img = image::open(&path).map_err(|e| format!("打开图片失败: {}", e))?;
    let rgba = img.to_rgba8();
    let results = bardecoder::default_decoder().decode(&rgba);
    for result in results {
        if let Ok(text) = result {
            return Ok(text);
        }
    }
    Err("未检测到二维码，请确保图片清晰且二维码完整".into())
}
