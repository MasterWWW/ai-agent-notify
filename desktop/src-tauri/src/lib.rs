mod commands;

use commands::Managed;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Build server state and start the in-process HTTP/WS server + Feishu
            // bot. `build_state`/`start_feishu_bot` spawn Tokio tasks, so the
            // whole init must run inside the Tauri async runtime (not the raw
            // setup thread, which has no Tokio context).
            let managed = tauri::async_runtime::block_on(async {
                // Resolve the token and build the in-process server state.
                let (token, _source) = ai_task_notify_server::state::resolve_token(None);
                let server_state = ai_task_notify_server::build_state(token);

                let managed = Managed {
                    server_state: server_state.clone(),
                    server: Mutex::new(None),
                    port: Mutex::new(3210),
                    bot: Mutex::new(None),
                };

                // Start the HTTP/WS server in-process (the hooks/phone talk to it).
                match ai_task_notify_server::http::serve(server_state.clone(), "0.0.0.0", 3210, true)
                    .await
                {
                    Ok(running) => {
                        *managed.port.lock().unwrap() = running.port;
                        *managed.server.lock().unwrap() = Some(running);
                    }
                    Err(e) => {
                        eprintln!("server start failed: {e}");
                    }
                }

                // Start the Feishu long-connection bot if configured.
                let bot = ai_task_notify_server::start_feishu_bot_component(&server_state);
                *managed.bot.lock().unwrap() = bot;

                managed
            });

            app.manage(managed);

            // Menu bar app: no Dock icon on macOS.
            #[cfg(target_os = "macos")]
            {
                use tauri::ActivationPolicy;
                let _ = app.set_activation_policy(ActivationPolicy::Accessory);
            }

            setup_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::get_config,
            commands::save_feishu_config,
            commands::feishu_test,
            commands::feishu_chats,
            commands::copy_text,
            commands::open_state_dir,
            commands::start_server,
            commands::stop_server,
            commands::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "打开/隐藏面板", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 AI Task Notify", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &quit])?;

    // Menu bar template icon: monochrome silhouette, auto-adapts to light/dark.
    let icon = {
        let bytes = include_bytes!("../icons/tray-icon@2x.png");
        tauri::image::Image::from_bytes(bytes)
            .map_err(|_| tauri::Error::AssetNotFound("tray-icon@2x.png".into()))?
    };

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("AI Task Notify")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => toggle_window(app),
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
                toggle_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn toggle_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            position_near_tray(&win);
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

/// Position the window in the top-right of the primary monitor, just below the
/// menu bar (close to the tray icon).
fn position_near_tray(win: &tauri::WebviewWindow) {
    if let Ok(Some(mon)) = win.current_monitor() {
        let msize = mon.size();
        let mpos = mon.position();
        let wsize = win.outer_size().unwrap_or(tauri::PhysicalSize::new(420, 640));
        let x = mpos.x + msize.width as i32 - wsize.width as i32 - 20;
        let y = mpos.y + 24;
        let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
    }
}
