fn main() {
    println!("cargo:rerun-if-env-changed=MEOWLIVE_RELEASE_TAG");
    println!("cargo:rerun-if-env-changed=MEOWLIVE_UPDATE_PUBLIC_KEY");
    #[cfg(windows)]
    tauri_build::build();
}
