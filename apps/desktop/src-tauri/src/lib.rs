mod plugins;
mod supervisor;

use supervisor::HarnessSupervisor;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, Url, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_deep_link::DeepLinkExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // A second launch focuses the existing window instead of starting a
        // second harness + server. Deep links are routed through the deep-link
        // plugin: single-instance (registered first, with the `deep-link`
        // feature) forwards dsh:// CLI arguments to it on Linux/Windows.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            plugins::marketplace_install,
            plugins::marketplace_preview,
            plugins::marketplace_apply,
            plugins::marketplace_undo,
            plugins::marketplace_reset,
            plugins::marketplace_has_candidate,
            plugins::marketplace_has_previous,
        ])
        .setup(|app| {
            // The main window is created in code so navigation can be filtered:
            // only the local connecting page and the dsh loopback origin may
            // load in-app; every other http(s) URL opens in the system browser.
            let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("DSH Desktop")
                .inner_size(1280.0, 820.0)
                .min_inner_size(900.0, 600.0)
                .center()
                .on_navigation(|url| {
                    if is_allowed_navigation(url) {
                        true
                    } else {
                        open_external(url);
                        false
                    }
                })
                .on_new_window(|url, _features| {
                    open_external(&url);
                    tauri::webview::NewWindowResponse::Deny
                })
                .build()?;

            // Closing the window hides it to the tray; only the tray Quit
            // terminates the harness and exits.
            let hide_target = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = hide_target.hide();
                }
            });

            build_tray(app)?;

            // Deep-link handler: focus the main window and re-emit the dsh://
            // URL as a `deep-link` event (frontend / dsh web UI may subscribe).
            #[cfg(desktop)]
            {
                let app_handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    if let Some(url) = event.urls().first() {
                        forward_deep_link(&app_handle, url);
                    }
                });
            }

            let supervisor = HarnessSupervisor::new();
            supervisor.start(app.handle().clone());
            app.manage(supervisor);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Opens (or focuses) the standalone plugin marketplace window.
fn open_market_window(app: &tauri::AppHandle) {
    if let Some(existing) = app.get_webview_window("market") {
        let _ = existing.show();
        let _ = existing.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "market", WebviewUrl::App("market.html".into()))
        .title("插件市场")
        .inner_size(820.0, 720.0)
        .min_inner_size(600.0, 480.0)
        .center()
        .build();
}

/// Creates the system tray with a native menu: show the main window, open the
/// plugin marketplace, or quit (which kills the harness process tree first).
fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItemBuilder::with_id("show", "显示主窗口").build(app)?;
    let market = MenuItemBuilder::with_id("market", "打开插件市场").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&show, &market, &quit])
        .build()?;

    let mut tray = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "market" => open_market_window(app),
            "quit" => {
                app.state::<HarnessSupervisor>().shutdown();
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
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

/// True when the URL may load inside the app webview: the Tauri internal
/// scheme or the loopback origin served by the dsh web child process.
fn is_allowed_navigation(url: &Url) -> bool {
    match url.scheme() {
        "tauri" => true,
        "http" | "https" => matches!(url.host_str(), Some("127.0.0.1") | Some("localhost")),
        _ => false,
    }
}

/// Opens an external URL in the system browser.
fn open_external(url: &Url) {
    if matches!(url.scheme(), "http" | "https") {
        let _ = tauri_plugin_opener::open_url(url.as_str(), None::<&str>);
    }
}

/// Focuses the main window and re-emits a dsh:// deep link so the frontend
/// (or the dsh web UI) can act on it.
fn forward_deep_link(app: &tauri::AppHandle, url: &Url) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    let _ = app.emit("deep-link", url.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_tauri_and_loopback_only() {
        assert!(is_allowed_navigation(&Url::parse("tauri://localhost/index.html").unwrap()));
        assert!(is_allowed_navigation(&Url::parse("http://127.0.0.1:8298/").unwrap()));
        assert!(is_allowed_navigation(&Url::parse("http://localhost:3080/").unwrap()));
        assert!(!is_allowed_navigation(&Url::parse("https://example.com/").unwrap()));
        assert!(!is_allowed_navigation(&Url::parse("https://github.com/x/y").unwrap()));
        assert!(!is_allowed_navigation(&Url::parse("file:///etc/passwd").unwrap()));
    }
}
