fn main() {
    println!("cargo::rerun-if-changed=assets/fetch.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("assets/fetch.ico")
        .set("ProductName", "Fetch")
        .set("FileDescription", "Fetch local media downloader")
        .set("InternalName", "fetch.exe")
        .set("OriginalFilename", "fetch.exe");
    resource
        .compile()
        .expect("could not compile Fetch Windows resources");
}
