fn main() {
    let app_manifest = tauri_build::AppManifest::new().commands(&["app_get_bootstrap"]);
    let attributes = tauri_build::Attributes::new().app_manifest(app_manifest);

    tauri_build::try_build(attributes).expect("failed to build Tauri application metadata");
}
