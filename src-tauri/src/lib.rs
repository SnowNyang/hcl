use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::UpdaterExt;
use tauri_plugin_window_state::{Builder as WindowStateBuilder, StateFlags};

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn toggle_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let visible = w.is_visible().unwrap_or(false);
        let minimized = w.is_minimized().unwrap_or(false);
        if visible && !minimized {
            let _ = w.hide();
        } else {
            show_main(app);
        }
    }
}

// ---- 자동 실행 ----
#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) {
    let m = app.autolaunch();
    if enabled {
        let _ = m.enable();
    } else {
        let _ = m.disable();
    }
}

// 설치 후 처음 실행할 때만 호출: 자동 실행을 기본으로 켬 (개발 중에는 건드리지 않음)
#[tauri::command]
fn init_autostart(app: AppHandle) {
    if !cfg!(debug_assertions) {
        let _ = app.autolaunch().enable();
    }
}

// ---- 창 ----
#[tauri::command]
fn win_min(window: tauri::WebviewWindow) {
    let _ = window.minimize();
}

#[tauri::command]
fn win_hide(window: tauri::WebviewWindow) {
    let _ = window.hide();
}

#[tauri::command]
fn set_always_on_top(window: tauri::WebviewWindow, enabled: bool) {
    let _ = window.set_always_on_top(enabled);
}

// ---- 전역 단축키 (창 보이기/숨기기) ----
#[tauri::command]
fn set_shortcut(app: AppHandle, shortcut: String, enabled: bool) -> Result<(), String> {
    let gs = app.global_shortcut();
    gs.unregister_all().map_err(|e| e.to_string())?;
    if enabled {
        gs.register(shortcut.as_str()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ---- 알림 ----
#[tauri::command]
fn notify(app: AppHandle, title: String, body: String) {
    let _ = app.notification().builder().title(title).body(body).show();
}

// ---- 백업 저장 (다운로드 폴더) ----
#[tauri::command]
fn export_data(app: AppHandle, json: String, name: String) -> Result<String, String> {
    let dir = app.path().download_dir().map_err(|e| e.to_string())?;
    let safe: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();
    let path = dir.join(if safe.is_empty() { "HCL-backup.json".to_string() } else { safe });
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

// ---- 게임 실행 바로가기 ----
#[tauri::command]
async fn pick_exe(app: AppHandle) -> Option<String> {
    let p = app
        .dialog()
        .file()
        .add_filter("실행 파일 / 바로가기", &["exe", "lnk", "bat", "url"])
        .blocking_pick_file()?;
    p.into_path().ok().map(|x| x.to_string_lossy().to_string())
}

#[tauri::command]
fn launch(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    if !p.exists() {
        return Err("파일을 찾을 수 없어요".into());
    }
    let mut cmd = std::process::Command::new("cmd");
    cmd.args(["/C", "start", "", &path]);
    if let Some(dir) = p.parent() {
        cmd.current_dir(dir);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // 검은 창 숨김
    }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

// ---- 자동 업데이트 ----
#[tauri::command]
fn get_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

// 새 버전이 있으면 버전 문자열을, 없으면 None을 돌려줌
#[tauri::command]
async fn check_update(app: AppHandle) -> Result<Option<String>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await.map_err(|e| e.to_string())? {
        Some(u) => Ok(Some(u.version.clone())),
        None => Ok(None),
    }
}

// 새 버전을 내려받아 설치하고 앱을 다시 시작
#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    if let Some(u) = updater.check().await.map_err(|e| e.to_string())? {
        u.download_and_install(|_chunk, _total| {}, || {})
            .await
            .map_err(|e| e.to_string())?;
        app.restart();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 이미 실행 중이면 새로 켜지 않고 기존 창을 보여줌 (반드시 첫 번째 플러그인)
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
        }))
        // 윈도우 시작 시 자동 실행 (켜져 있으면 창을 숨기고 트레이로 시작)
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_main(app);
                    }
                })
                .build(),
        )
        // 창 위치·크기 기억
        .plugin(
            WindowStateBuilder::default()
                .with_state_flags(StateFlags::SIZE | StateFlags::POSITION)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_autostart,
            set_autostart,
            init_autostart,
            win_min,
            win_hide,
            set_always_on_top,
            set_shortcut,
            notify,
            export_data,
            pick_exe,
            launch,
            get_version,
            check_update,
            install_update
        ])
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "열기", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("HCL")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            // 자동 실행으로 켜졌을 때는 창을 숨기고 트레이에서 시작
            if std::env::args().any(|a| a == "--autostart") {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            Ok(())
        })
        // X(닫기)를 누르면 종료하지 않고 트레이로 숨김
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("앱 실행 중 오류가 발생했습니다");
}
