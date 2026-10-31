fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon.ico");
        res.set("ProductName", "MouseBlur");
        res.set("FileDescription", "Hardware-Accelerated Mouse Motion Blur for Windows");
        res.set("LegalCopyright", "Copyright (c) 2026 MouseBlur");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Failed to compile Windows resource: {:?}", e);
        }
    }
}
