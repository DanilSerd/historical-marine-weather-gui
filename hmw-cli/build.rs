fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    winres::WindowsResource::new()
        .compile()
        .expect("windows resources compile");
}
