pub mod application;
pub mod domain;
pub mod infrastructure;
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

#[cfg(test)]
mod architecture_boundary {
    use std::path::{Path, PathBuf};

    const FORBIDDEN_DOMAIN_DEPENDENCIES: [&str; 3] = ["tauri", "sqlx", "reqwest"];
    const FORBIDDEN_APPLICATION_RUNTIME_DEPENDENCIES: [&str; 1] = ["tauri"];

    #[test]
    fn domain_is_independent_of_runtime_and_infrastructure_crates() {
        assert_sources_exclude("domain", &FORBIDDEN_DOMAIN_DEPENDENCIES);
    }

    #[test]
    fn application_translation_boundary_is_importable_without_a_runtime() {
        assert_sources_exclude(
            "application/translation",
            &FORBIDDEN_APPLICATION_RUNTIME_DEPENDENCIES,
        );
    }

    fn assert_sources_exclude(relative_module: &str, forbidden_dependencies: &[&str]) {
        let module_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join(relative_module);
        assert!(
            module_root.is_dir(),
            "architecture module must exist: {}",
            module_root.display()
        );

        let sources = rust_sources(&module_root);
        assert!(
            !sources.is_empty(),
            "architecture module must contain Rust sources: {}",
            module_root.display()
        );

        for source_path in sources {
            let source = std::fs::read_to_string(&source_path).unwrap_or_else(|error| {
                panic!("failed to read {}: {error}", source_path.display())
            });

            for forbidden_dependency in forbidden_dependencies {
                assert!(
                    !source.contains(forbidden_dependency),
                    "{} must not depend on {forbidden_dependency}",
                    source_path.display()
                );
            }
        }
    }

    fn rust_sources(module_root: &Path) -> Vec<PathBuf> {
        let mut pending = vec![module_root.to_path_buf()];
        let mut sources = Vec::new();

        while let Some(path) = pending.pop() {
            let entries = std::fs::read_dir(&path)
                .unwrap_or_else(|error| panic!("failed to inspect {}: {error}", path.display()));

            for entry in entries {
                let entry = entry.unwrap_or_else(|error| {
                    panic!("failed to inspect an entry in {}: {error}", path.display())
                });
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    pending.push(entry_path);
                } else if entry_path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    == Some("rs")
                {
                    sources.push(entry_path);
                }
            }
        }

        sources.sort();
        sources
    }
}
