use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppBootstrap {
    pub(crate) name: &'static str,
    pub(crate) version: &'static str,
    pub(crate) platform: &'static str,
    pub(crate) architecture: &'static str,
}
