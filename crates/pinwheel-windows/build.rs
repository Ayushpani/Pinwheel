fn main() {
    // The icon and the manifest (per-monitor DPI, modern controls) go into
    // the .exe on Windows only.
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        embed_resource::compile("assets/pinwheel.rc", embed_resource::NONE)
            .manifest_optional()
            .expect("embed the icon and manifest");
    }
}
