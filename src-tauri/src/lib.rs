mod application;
pub mod ipc;

/// Starts the Evolish desktop runtime and blocks until the application exits.
///
/// # Panics
///
/// Panics when the Tauri runtime cannot be initialized or encounters an unrecoverable error.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ipc::bootstrap::app_get_bootstrap])
        .run(tauri::generate_context!())
        .expect("failed to run Evolish desktop application");
}
