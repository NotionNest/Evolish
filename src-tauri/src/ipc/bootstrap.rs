use crate::{application, ipc::dto::AppBootstrapDto};

#[tauri::command]
pub(crate) fn app_get_bootstrap() -> AppBootstrapDto {
    application::app_bootstrap().into()
}
