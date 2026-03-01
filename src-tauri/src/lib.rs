pub mod automation;
pub mod ocr;

use automation::AutomationManager;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, Position, PhysicalPosition, Size, PhysicalSize};
use tauri::tray::TrayIconBuilder;
use tauri::menu::{Menu, MenuItem, CheckMenuItem};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering, AtomicU64};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use windows::Win32::Foundation::POINT;

const TRIGGER_THRESHOLD: f64 = 120.0;
const HIDE_THRESHOLD: f64 = 180.0;
const INTERACTION_OFFSET_Y: f64 = 40.0;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct WormlinkMatch {
    pub url: String,
    pub source_app: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub trigger_x: f64,
    pub trigger_y: f64,
    pub trigger_w: f64,
    pub trigger_h: f64,
    pub is_demo: bool,
    pub detection_method: String,
    pub parent_x: f64,
    pub parent_y: f64,
    pub parent_width: f64,
    pub parent_height: f64,
}

#[derive(Default)]
struct AppState {
    active_match: Mutex<Option<WormlinkMatch>>,
    last_injected_url: Mutex<String>,
    muted_urls: Mutex<std::collections::HashMap<String, u64>>,
    use_ui_automation: AtomicBool,
    is_maximized: AtomicBool,
    is_pinned: AtomicBool,
    last_toggle_time: AtomicU64,
    last_dismiss_time: AtomicU64,
    is_visible: AtomicBool,
}

fn dist_to_rect(px: f64, py: f64, rx: f64, ry: f64, rw: f64, rh: f64) -> f64 {
    let dx = (rx - px).max(0.0).max(px - (rx + rw));
    let dy = (ry - py).max(0.0).max(py - (ry + rh));
    (dx * dx + dy * dy).sqrt()
}

#[tauri::command]
fn get_wormlink_data(state: tauri::State<Arc<AppState>>) -> Option<WormlinkMatch> {
    state.active_match.lock().unwrap().clone()
}

#[tauri::command]
fn dismiss_portal(app: AppHandle, state: tauri::State<Arc<AppState>>) {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    state.last_dismiss_time.store(now, Ordering::Relaxed);
    state.is_pinned.store(false, Ordering::Relaxed); 
    
    // Mute this specific URL for 10 seconds
    let current_url = {
        let active = state.active_match.lock().unwrap();
        active.as_ref().map(|m| m.url.clone())
    };

    if let Some(url) = current_url {
        let mut muted = state.muted_urls.lock().unwrap();
        muted.insert(url, now + 10);
    }
    
    // Clear the last injected URL so it can be re-triggered immediately after cooldown (if it's a different one)
    {
        let mut last_url = state.last_injected_url.lock().unwrap();
        *last_url = String::new();
    }
    
    if let Some(win) = app.get_webview_window("active_portal") {
        let _ = win.hide();
        state.is_visible.store(false, Ordering::Relaxed);
        let mut active = state.active_match.lock().unwrap();
        *active = None;
    }
}

#[tauri::command]
fn toggle_pin(state: tauri::State<Arc<AppState>>) -> bool {
    let current = state.is_pinned.load(Ordering::Relaxed);
    let new_state = !current;
    state.is_pinned.store(new_state, Ordering::Relaxed);
    new_state
}

#[tauri::command]
fn toggle_maximize(app: AppHandle, state: tauri::State<Arc<AppState>>) {
    let current = state.is_maximized.load(Ordering::Relaxed);
    let new_state = !current;
    state.is_maximized.store(new_state, Ordering::Relaxed);
    
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    state.last_toggle_time.store(now, Ordering::Relaxed);

    if let Some(win) = app.get_webview_window("active_portal") {
        if let Some(m) = state.active_match.lock().unwrap().clone() {
            if new_state {
                let _ = win.set_position(Position::Physical(PhysicalPosition { x: m.parent_x as i32, y: m.parent_y as i32 }));
                let _ = win.set_size(Size::Physical(PhysicalSize { width: m.parent_width as u32, height: m.parent_height as u32 }));
            } else {
                let _ = win.set_position(Position::Physical(PhysicalPosition { x: m.x as i32, y: m.y as i32 }));
                let _ = win.set_size(Size::Physical(PhysicalSize { width: m.width as u32, height: m.height as u32 }));
            }
            let js = format!("if(window.updateMaximizeState) {{ window.updateMaximizeState({}); }}", new_state);
            let _ = win.eval(&js);
        }
    }
}

pub fn run() {
    tracing_subscriber::fmt::init();
    let app_state = Arc::new(AppState::default());
    app_state.use_ui_automation.store(true, Ordering::Relaxed);

    let setup_state = app_state.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .register_asynchronous_uri_scheme_protocol("wormhole", |_app, request, responder| {
            let uri = request.uri().to_string();
            // Handle both wormhole://localhost/URL and wormhole://localhostURL
            let url_str = if uri.contains("localhost/") {
                uri.splitn(2, "localhost/").nth(1).unwrap_or("")
            } else {
                uri.strip_prefix("wormhole://localhost").unwrap_or(&uri)
            };
            
            let target_url = match urlencoding::decode(url_str) {
                Ok(u) => u.to_string(),
                Err(_) => url_str.to_string(),
            };

            tauri::async_runtime::spawn(async move {
                if target_url.is_empty() || !target_url.starts_with("http") {
                    let _ = responder.respond(tauri::http::Response::builder().status(400).body(Vec::new()).unwrap());
                    return;
                }
                
                let client = reqwest::Client::new();
                match client.get(&target_url).header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36").send().await {
                    Ok(resp) => {
                        let content_type = resp.headers().get("content-type").and_then(|h: &reqwest::header::HeaderValue| h.to_str().ok()).unwrap_or("text/html").to_string();
                        let bytes = resp.bytes().await.unwrap_or_default();
                        
                        let tauri_resp = tauri::http::Response::builder()
                            .header("Content-Type", content_type)
                            .header("Access-Control-Allow-Origin", "*")
                            .header("Content-Security-Policy", "default-src * 'unsafe-inline' 'unsafe-eval'; frame-ancestors *")
                            .body(bytes.to_vec())
                            .unwrap();
                        let _ = responder.respond(tauri_resp);
                    }
                    Err(_) => {
                        let _ = responder.respond(tauri::http::Response::builder().status(404).body(Vec::new()).unwrap());
                    }
                }
            });
        })
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![get_wormlink_data, toggle_maximize, dismiss_portal, toggle_pin])
        .setup(move |app| {
            let automation_toggle = CheckMenuItem::with_id(app, "toggle_automation", "Native UI Intelligence", true, true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&automation_toggle, &quit_i])?;
            
            let tray_state = setup_state.clone();
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(move |app, event| {
                    if event.id == tauri::menu::MenuId::new("quit") { app.exit(0); }
                    else if event.id == tauri::menu::MenuId::new("toggle_automation") {
                        let current = tray_state.use_ui_automation.load(Ordering::Relaxed);
                        tray_state.use_ui_automation.store(!current, Ordering::Relaxed);
                    }
                })
                .build(app)?;

            let app_handle = app.handle().clone();
            let poller_state = setup_state.clone();
            
            tauri::async_runtime::spawn(async move {
                let automation = AutomationManager::new().ok();
                loop {
                    let now_secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                    let last_toggle = poller_state.last_toggle_time.load(Ordering::Relaxed);
                    let last_dismiss = poller_state.last_dismiss_time.load(Ordering::Relaxed);
                    let in_cooldown = (now_secs - last_toggle) < 1 || (now_secs - last_dismiss) < 1;

                    if poller_state.use_ui_automation.load(Ordering::Relaxed) && !poller_state.is_maximized.load(Ordering::Relaxed) && !in_cooldown {
                        let mut cursor_pos = POINT::default();
                        unsafe { let _ = GetCursorPos(&mut cursor_pos); }

                        let is_pinned = poller_state.is_pinned.load(Ordering::Relaxed);

                        // If pinned, skip all detection to lock onto the current link
                        if is_pinned {
                            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
                            continue;
                        }

                        // Interaction lock: freeze if mouse is in portal
                        if let Some(win) = app_handle.get_webview_window("active_portal") {
                            if let (Ok(pos), Ok(size), Ok(visible)) = (win.outer_position(), win.outer_size(), win.is_visible()) {
                                if visible && 
                                   cursor_pos.x >= pos.x && cursor_pos.x <= (pos.x + size.width as i32) &&
                                   cursor_pos.y >= pos.y && cursor_pos.y <= (pos.y + size.height as i32) {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
                                    continue;
                                }
                            }
                        }

                        if let Some(ref am) = automation {
                            let matches = am.find_matches_near_cursor(cursor_pos);
                            
                            // Filter out muted URLs
                            let mut filtered_matches = Vec::new();
                            {
                                let muted = poller_state.muted_urls.lock().unwrap();
                                for m in matches {
                                    if let Some(expiry) = muted.get(&m.url) {
                                        if now_secs < *expiry { continue; }
                                    }
                                    filtered_matches.push(m);
                                }
                            }

                            let closest = filtered_matches.into_iter().min_by_key(|m| {
                                let d = dist_to_rect(cursor_pos.x as f64, cursor_pos.y as f64, m.trigger_x, m.trigger_y, m.trigger_w, m.trigger_h);
                                (d * 100.0) as i64
                            });

                            let currently_visible = poller_state.is_visible.load(Ordering::Relaxed);
                            let threshold = if currently_visible { HIDE_THRESHOLD } else { TRIGGER_THRESHOLD };

                            if let Some(mut m) = closest {
                                m.y += INTERACTION_OFFSET_Y;
                                let d = dist_to_rect(cursor_pos.x as f64, cursor_pos.y as f64, m.trigger_x, m.trigger_y, m.trigger_w, m.trigger_h);
                                
                                if d < threshold {
                                    let content_changed = {
                                        let mut last_url = poller_state.last_injected_url.lock().unwrap();
                                        if *last_url == m.url { false }
                                        else { *last_url = m.url.clone(); true }
                                    };
                                    
                                    let pos_changed = {
                                        let active = poller_state.active_match.lock().unwrap();
                                        match &*active {
                                            Some(old) => (old.x - m.x).abs() > 10.0 || (old.y - m.y).abs() > 10.0,
                                            None => true
                                        }
                                    };

                                    if content_changed || pos_changed || !currently_visible {
                                        update_active_portal(&app_handle, &poller_state, m, content_changed).await;
                                    }
                                } else if currently_visible { hide_portal(&app_handle, &poller_state).await; }
                            } else if currently_visible { hide_portal(&app_handle, &poller_state).await; }
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn update_active_portal(app: &AppHandle, state: &AppState, m: WormlinkMatch, content_changed: bool) {
    let win_label = "active_portal";

    { let mut active = state.active_match.lock().unwrap(); *active = Some(m.clone()); }

    if let Some(win) = app.get_webview_window(win_label) {
        let _ = win.set_position(Position::Physical(PhysicalPosition { x: m.x as i32, y: m.y as i32 }));
        if content_changed {
            let safe_url = serde_json::to_string(&m.url).unwrap_or_default();
            let safe_app = serde_json::to_string(&m.source_app).unwrap_or_default();
            let js = format!("if(window.updatePortalContent) {{ window.updatePortalContent({}, {}, {}); }}", safe_url, safe_app, m.is_demo);
            let _ = win.eval(&js);
        }
        if !state.is_visible.load(Ordering::Relaxed) {
            let _ = win.show();
            state.is_visible.store(true, Ordering::Relaxed);
        }
    } else {
        let builder = WebviewWindowBuilder::new(app, win_label, WebviewUrl::App("index.html".into()))
            .title("Wormlinks Portal").inner_size(m.width, m.height).transparent(true).decorations(false).always_on_top(true).skip_taskbar(true);
        if let Ok(window) = builder.build() {
            let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x: m.x as i32, y: m.y as i32 }));
            state.is_visible.store(true, Ordering::Relaxed);
        }
    }
}

async fn hide_portal(app: &AppHandle, state: &AppState) {
    if state.is_visible.load(Ordering::Relaxed) {
        if let Some(win) = app.get_webview_window("active_portal") {
            let _ = win.hide();
            state.is_visible.store(false, Ordering::Relaxed);
            let mut last_url = state.last_injected_url.lock().unwrap();
            *last_url = String::new();
            let mut active = state.active_match.lock().unwrap();
            *active = None;
        }
    }
}

