// main.rs — SnipChat：托盘 + 全局热键 + 框选浮层/编辑器窗口管理
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;
mod codex;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            capture::start_region,
            capture::start_full,
            capture::get_capture,
            capture::handoff_crop,
            capture::save_result,
            codex::codex_send,
            codex::codex_mirror_transcript,
        ])
        .setup(|app| {
            // 托盘菜单
            let region = MenuItem::with_id(app, "region", "区域截屏  Ctrl+Alt+A", true, None::<&str>)?;
            let full = MenuItem::with_id(app, "full", "全屏截屏  Ctrl+Alt+S", true, None::<&str>)?;
            let panel = MenuItem::with_id(app, "panel", "打开主面板", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&region, &full, &panel, &quit])?;

            TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("SnipChat · Ctrl+Alt+A 截屏")
                .on_menu_event(|app, ev| match ev.id.as_ref() {
                    "region" => {
                        let _ = capture::start_region(app.clone());
                    }
                    "full" => {
                        let _ = capture::start_full(app.clone());
                    }
                    "panel" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // 全局热键
            use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
            let gs = app.global_shortcut();
            let sc_a = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyA);
            let sc_s = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyS);
            let app2 = app.handle().clone();
            gs.on_shortcut(sc_a, move |_, _, event| {
                if event.state() == ShortcutState::Pressed {
                    let _ = capture::start_region(app2.clone());
                }
            })?;
            let app3 = app.handle().clone();
            gs.on_shortcut(sc_s, move |_, _, event| {
                if event.state() == ShortcutState::Pressed {
                    let _ = capture::start_full(app3.clone());
                }
            })?;

            // 框选浮层：透明置顶全屏
            WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("overlay.html".into()))
                .title("overlay")
            .fullscreen(true)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .build()?;

            // 编辑器：隐藏待命
            WebviewWindowBuilder::new(app, "editor", WebviewUrl::App("editor.html".into()))
                .title("SnipChat · 标注")
            .decorations(false)
            .resizable(true)
            .visible(false)
            .build()?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // 主窗关闭 → 收进托盘
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running snipchat");
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
        let _ = app.emit("chat:focus", ());
    }
}
