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

#[cfg(test)]
mod tests {
    use super::AppErrorDto;

    #[test]
    fn serializes_a_stable_safe_error_contract() {
        let error = AppErrorDto {
            code: "ipc.unexpected".to_owned(),
            message_key: "error.ipc.unavailable".to_owned(),
            retryable: true,
            suggested_action: Some("retry".to_owned()),
            source: Some("ipc".to_owned()),
            diagnostic_detail: Some("transport closed".to_owned()),
        };

        let serialized = serde_json::to_value(error).expect("error DTO must serialize");

        assert_eq!(serialized["code"], "ipc.unexpected");
        assert_eq!(serialized["messageKey"], "error.ipc.unavailable");
        assert_eq!(serialized["retryable"], true);
        assert_eq!(serialized["suggestedAction"], "retry");
        assert!(serialized.get("cause").is_none());
    }
}
