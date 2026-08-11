#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AppMetadata {
    pub(crate) name: &'static str,
    pub(crate) version: &'static str,
    pub(crate) platform: &'static str,
    pub(crate) architecture: &'static str,
}

pub(crate) fn app_bootstrap() -> AppMetadata {
    AppMetadata {
        name: "Evolish",
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
    }
}

#[cfg(test)]
mod tests {
    use super::app_bootstrap;
    use crate::ipc::dto::AppBootstrapDto;

    #[test]
    fn returns_build_and_platform_metadata() {
        let bootstrap = app_bootstrap();

        assert_eq!(bootstrap.name, "Evolish");
        assert_eq!(bootstrap.version, env!("CARGO_PKG_VERSION"));
        assert!(!bootstrap.platform.is_empty());
        assert!(!bootstrap.architecture.is_empty());
    }

    #[test]
    fn maps_internal_metadata_to_the_public_ipc_contract() {
        let metadata = app_bootstrap();
        let dto = AppBootstrapDto::from(metadata);

        assert_eq!(dto.name, "Evolish");
        assert_eq!(dto.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(dto.platform, std::env::consts::OS);
        assert_eq!(dto.architecture, std::env::consts::ARCH);
    }
}
