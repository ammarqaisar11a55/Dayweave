mod autostart;
mod instance;
mod notifier;
mod scheduler;
mod tray;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::Thread;
use std::time::Duration;

use dayweave_core::{Core, Error};
use instance::Instance;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

pub struct Shell {
    core: Arc<Core>,
    scheduler: Thread,
    /// A launcher action that arrived before the interface was ready for it.
    pending: Mutex<Option<String>>,
    ready: AtomicBool,
    background: bool,
}

pub(crate) fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Runs a launcher, tray or second-launch action in the interface.
pub(crate) fn perform(app: &AppHandle, action: &str) {
    show_window(app);
    let shell = app.state::<Shell>();
    if shell.ready.load(Ordering::SeqCst) {
        let _ = app.emit("shell-action", action);
    } else {
        *shell.pending.lock().unwrap() = Some(action.to_string());
    }
}

#[tauri::command]
async fn rpc(state: State<'_, Shell>, method: String, params: Value) -> Result<Value, Error> {
    let core = state.core.clone();
    let wakes_scheduler = method.starts_with("focus.");
    let autostart = (method == "settings.set" && params["key"] == "startAtLogin")
        .then(|| params["value"].as_bool().unwrap_or(false));

    let result = tauri::async_runtime::spawn_blocking(move || core.rpc(&method, params))
        .await
        .map_err(|error| Error::invalid(format!("The request was interrupted: {error}")))?;
    if wakes_scheduler {
        state.scheduler.unpark();
    }
    let value = result?;
    if let Some(enabled) = autostart {
        autostart::set(enabled)?;
    }
    Ok(value)
}

/// Called once the interface has rendered, so the window never opens blank. Returns any action
/// waiting for it.
#[tauri::command]
fn interface_ready(app: AppHandle, state: State<'_, Shell>) -> Option<String> {
    state.ready.store(true, Ordering::SeqCst);
    if !state.background {
        show_window(&app);
    }
    state.pending.lock().unwrap().take()
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

fn action_from_arguments() -> (String, bool) {
    let mut action = "show".to_string();
    let mut background = false;
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--background" => background = true,
            "--new-task" => action = "new-task".into(),
            "--new-habit" => action = "new-habit".into(),
            "--start-focus" => action = "start-focus".into(),
            _ => {}
        }
    }
    (action, background)
}

fn open_core(app: &AppHandle) -> Result<Arc<Core>, String> {
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?;
    Core::open(&directory).map(Arc::new).map_err(|error| error.to_string())
}

pub fn run() {
    let (action, background) = action_from_arguments();
    let listener = match instance::acquire(&action) {
        Instance::Forwarded => return,
        Instance::Primary(listener) => Some(listener),
        Instance::Unavailable => None,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![rpc, interface_ready, quit])
        .setup(move |app| {
            let handle = app.handle().clone();
            let core = match open_core(&handle) {
                Ok(core) => core,
                Err(message) => {
                    handle
                        .dialog()
                        .message(message)
                        .title("Dayweave could not open your data")
                        .kind(MessageDialogKind::Error)
                        .blocking_show();
                    std::process::exit(1);
                }
            };

            let scheduler = scheduler::start(handle.clone(), core.clone());
            let pending = (action != "show").then(|| action.clone());
            app.manage(Shell {
                core,
                scheduler,
                pending: Mutex::new(pending),
                ready: AtomicBool::new(false),
                background,
            });

            if let Err(error) = tray::create(&handle) {
                eprintln!("no system tray: {error}");
            }
            if let Some(listener) = listener {
                let forwarded = handle.clone();
                instance::listen(listener, move |message| match message {
                    "show" => show_window(&forwarded),
                    action => perform(&forwarded, action),
                });
            }

            // If the interface failed to load, an open window with an error beats an invisible one.
            if !background {
                let fallback = handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(5));
                    show_window(&fallback);
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let shell = window.state::<Shell>();
                if shell.core.setting_bool("closeToBackground").unwrap_or(true) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("build the application")
        .run(|_, event| {
            // Staying alive with no window open is what keeps reminders running.
            if let RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
