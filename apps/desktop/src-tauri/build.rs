fn main() {
    println!("cargo:rerun-if-env-changed=MEOWLIVE_RELEASE_TAG");
    println!("cargo:rerun-if-env-changed=MEOWLIVE_UPDATE_PUBLIC_KEY");
    // Build scripts run on the host, which may be Linux when targeting Windows.
    // Tauri embeds the Common Controls v6 manifest and icons for the target exe.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        tauri_build::build();
    }
}
