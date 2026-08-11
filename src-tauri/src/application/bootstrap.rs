use crate::domain::AppBootstrap;

pub(crate) fn app_bootstrap() -> AppBootstrap {
    AppBootstrap {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
    }
}

#[cfg(test)]
mod tests {
    use super::app_bootstrap;

    #[test]
    fn returns_build_and_platform_metadata() {
        let bootstrap = app_bootstrap();

        assert_eq!(bootstrap.name, "evolish");
        assert_eq!(bootstrap.version, env!("CARGO_PKG_VERSION"));
        assert!(!bootstrap.platform.is_empty());
        assert!(!bootstrap.architecture.is_empty());
    }
}
