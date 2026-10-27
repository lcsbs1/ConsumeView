mod db;
mod domain;
mod proxy;
mod secrets;

use chrono::{Datelike, Local};
use db::{default_db_path, MetricsDb, SessionInfo, StoredRequest};
use domain::{
    compute_overview, load_pricing, session_totals, OverviewMetrics, Period, PricingTable,
    SessionTotals,
};
use proxy::{ProxyState, BIND_ADDR};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WindowEvent,
};
use tokio::sync::RwLock;
use uuid::Uuid;

struct AppState {
    db: Arc<MetricsDb>,
    #[allow(dead_code)]
    pricing: Arc<PricingTable>,
    listening: Arc<AtomicBool>,
    active_session: Arc<RwLock<String>>,
}

#[derive(Serialize)]
struct ProxyStatus {
    listening: bool,
    bind: String,
}

#[tauri::command]
fn get_proxy_status(state: tauri::State<'_, AppState>) -> ProxyStatus {
    ProxyStatus {
        listening: state.listening.load(Ordering::Relaxed),
        bind: BIND_ADDR.to_string(),
    }
}

#[tauri::command]
fn get_key_status() -> secrets::KeyStatus {
    secrets::key_status()
}

#[tauri::command]
fn set_api_keys(gemini: Option<String>, xai: Option<String>) -> Result<(), String> {
    if let Some(g) = gemini {
        let g = g.trim();
        if g.is_empty() {
            secrets::clear_gemini_key().map_err(|e| e.to_string())?;
        } else {
            secrets::set_gemini_key(g).map_err(|e| e.to_string())?;
        }
    }
    if let Some(x) = xai {
        let x = x.trim();
        if x.is_empty() {
            secrets::clear_xai_key().map_err(|e| e.to_string())?;
        } else {
            secrets::set_xai_key(x).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
async fn list_sessions(state: tauri::State<'_, AppState>) -> Result<Vec<SessionInfo>, String> {
    state.db.list_sessions().map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_session(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<SessionInfo, String> {
    let id = format!("session-{}", Uuid::new_v4());
    let name = if name.trim().is_empty() {
        "New session".into()
    } else {
        name.trim().to_string()
    };
    state
        .db
        .create_session(&id, &name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_active_session(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(state.active_session.read().await.clone())
}

#[tauri::command]
async fn set_active_session(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    state
        .db
        .set_active_session(&session_id)
        .map_err(|e| e.to_string())?;
    *state.active_session.write().await = session_id;
    Ok(())
}

#[tauri::command]
async fn get_session_totals(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> Result<SessionTotals, String> {
    let rows = state
        .db
        .session_request_rows(&session_id)
        .map_err(|e| e.to_string())?;
    Ok(session_totals(&rows))
}

#[tauri::command]
async fn get_session_history(
    state: tauri::State<'_, AppState>,
    session_id: String,
    limit: Option<i64>,
) -> Result<Vec<StoredRequest>, String> {
    state
        .db
        .list_session_requests(&session_id, limit.unwrap_or(100))
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_overview(
    state: tauri::State<'_, AppState>,
    period: String,
) -> Result<OverviewMetrics, String> {
    let period = match period.as_str() {
        "week" => Period::Week,
        "month" => Period::Month,
        _ => Period::Today,
    };
    let now = Local::now();
    let since = match period {
        Period::Today => format!(
            "{:04}-{:02}-{:02} 00:00:00",
            now.year(),
            now.month(),
            now.day()
        ),
        Period::Week => (now - chrono::Duration::days(7))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        Period::Month => format!("{:04}-{:02}-01 00:00:00", now.year(), now.month()),
    };
    let rows = state
        .db
        .period_request_rows(&since)
        .map_err(|e| e.to_string())?;
    let mtd = state.db.month_to_date_cost().map_err(|e| e.to_string())?;
    let days_in_month = days_in_month(now.year(), now.month());
    Ok(compute_overview(
        &rows,
        period,
        now.day(),
        days_in_month,
        mtd,
    ))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    chrono::NaiveDate::from_ymd_opt(
        if month == 12 { year + 1 } else { year },
        if month == 12 { 1 } else { month + 1 },
        1,
    )
    .and_then(|d| d.pred_opt())
    .map(|d| d.day())
    .unwrap_or(30)
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let pricing_json = include_str!("../pricing.json");
    let pricing = Arc::new(load_pricing(pricing_json).expect("pricing.json"));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let db = Arc::new(MetricsDb::open(&default_db_path()).expect("open metrics db"));
            let active = db
                .get_active_session()
                .unwrap_or_else(|_| "cursor-workspace".into());
            let listening = Arc::new(AtomicBool::new(false));
            let active_session = Arc::new(RwLock::new(active));

            let proxy_state = ProxyState {
                db: db.clone(),
                pricing: pricing.clone(),
                app: app.handle().clone(),
                listening: listening.clone(),
                active_session: active_session.clone(),
            };

            tauri::async_runtime::spawn(async move {
                if let Err(e) = proxy::serve(proxy_state).await {
                    tracing::error!("proxy stopped: {e}");
                }
            });

            app.manage(AppState {
                db,
                pricing,
                listening,
                active_session,
            });

            // System tray
            let show_i = MenuItem::with_id(app, "show", "Show ConsumeView", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("ConsumeView — proxy 127.0.0.1:4000")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main_window(app),
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Default: hide to tray; proxy keeps running
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_proxy_status,
            get_key_status,
            set_api_keys,
            list_sessions,
            create_session,
            get_active_session,
            set_active_session,
            get_session_totals,
            get_session_history,
            get_overview,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ConsumeView");
}
