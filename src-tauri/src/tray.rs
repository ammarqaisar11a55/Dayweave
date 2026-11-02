use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

use crate::{perform, show_window};

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Dayweave", true, None::<&str>)?;
    let new_task = MenuItem::with_id(app, "new-task", "New task", true, None::<&str>)?;
    let new_habit = MenuItem::with_id(app, "new-habit", "New habit", true, None::<&str>)?;
    let focus = MenuItem::with_id(app, "start-focus", "Start focus session", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Dayweave", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &separator, &new_task, &new_habit, &focus, &separator, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Dayweave")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_window(app),
            "quit" => app.exit(0),
            action => perform(app, action),
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    let tray = tray.build(app)?;
    app.manage(tray);
    Ok(())
}
