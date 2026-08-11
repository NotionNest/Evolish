use crate::{application, domain::AppBootstrap};

#[tauri::command]
pub(crate) fn get_app_bootstrap() -> AppBootstrap {
    application::app_bootstrap()
}
