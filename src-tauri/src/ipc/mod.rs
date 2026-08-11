pub(crate) mod bootstrap;
pub mod dto;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::Value;
    use ts_rs::TS;

    use super::dto::{AppBootstrapDto, AppErrorDto};

    #[test]
    fn exports_frontend_contracts_to_the_generated_bridge_directory() {
        let export_config = ts_rs::Config::from_env();
        AppBootstrapDto::export(&export_config).expect("bootstrap contract must export");
        AppErrorDto::export(&export_config).expect("error contract must export");

        let generated = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/bridge/generated");

        assert!(generated.join("AppBootstrapDto.ts").is_file());
        assert!(generated.join("AppErrorDto.ts").is_file());
    }

    #[test]
    fn grants_bootstrap_only_to_the_main_window() {
        let capability_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("capabilities/default.json");
        let capability: Value = serde_json::from_str(
            &std::fs::read_to_string(capability_path).expect("capability must be readable"),
        )
        .expect("capability must be valid JSON");

        assert_eq!(capability["windows"], serde_json::json!(["main"]));

        let permissions = capability["permissions"]
            .as_array()
            .expect("permissions must be an array");
        assert!(
            permissions
                .iter()
                .any(|permission| { permission.as_str() == Some("allow-app-get-bootstrap") })
        );
        assert!(
            !permissions
                .iter()
                .any(|permission| { permission.as_str() == Some("deny-app-get-bootstrap") })
        );
    }
}
