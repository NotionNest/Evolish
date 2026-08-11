use serde::Serialize;
use ts_rs::TS;

use crate::application::AppMetadata;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bridge/generated/")]
pub struct AppBootstrapDto {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub architecture: String,
}

impl From<AppMetadata> for AppBootstrapDto {
    fn from(metadata: AppMetadata) -> Self {
        Self {
            name: metadata.name.to_owned(),
            version: metadata.version.to_owned(),
            platform: metadata.platform.to_owned(),
            architecture: metadata.architecture.to_owned(),
        }
    }
}
