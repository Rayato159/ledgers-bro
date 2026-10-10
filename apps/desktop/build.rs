fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=../../design/app-icon/launcher.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("../../design/app-icon/launcher.ico")
            .set("ProductName", "Ledgers Bro")
            .set("FileDescription", "Ledgers Bro")
            .set("CompanyName", "dancingwithmycode.com")
            .compile()?;
    }
    Ok(())
}
