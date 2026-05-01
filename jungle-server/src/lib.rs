/// jungle-server
pub fn hello() -> &'static str {
    concat!("Hello from ", env!("CARGO_PKG_NAME"), "!")
}
