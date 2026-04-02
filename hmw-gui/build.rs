fn main() {
    println!("cargo:rerun-if-changed=assets/logo.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winres::WindowsResource::new();
    resource.set_icon("assets/logo.ico");
    resource.compile().expect("windows resources compile");
}
