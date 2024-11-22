fn main() {
    slint_build::compile("ui/compact.slint").unwrap();
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("muted.ico");
        res.compile().unwrap();
    }
}
