use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bridge/generated/")]
pub struct AppErrorDto {
    pub code: String,
    pub message_key: String,
    pub retryable: bool,
    pub suggested_action: Option<String>,
    pub source: Option<String>,
    pub diagnostic_detail: Option<String>,
}
