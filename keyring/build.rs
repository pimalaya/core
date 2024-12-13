use std::fs;

fn main() {
    let _ = fs::remove_file("./src/secret_service_blocking/api.rs");
    let _ = fs::remove_file("./src/secret_service_nonblock/api.rs");

    let xml = include_str!("./org.freedesktop.Secrets.xml");

    let mut opts = dbus_codegen::GenOpts::default();
    opts.methodtype = None;

    #[cfg(feature = "secret-service-blocking")]
    generate_blocking_api(xml, &mut opts);

    #[cfg(feature = "secret-service-nonblock")]
    generate_nonblock_api(xml, &mut opts);
}

#[cfg(feature = "secret-service-blocking")]
fn generate_blocking_api(xml: &str, opts: &mut dbus_codegen::GenOpts) {
    opts.connectiontype = dbus_codegen::ConnectionType::Blocking;

    let api = dbus_codegen::generate(xml, &opts).expect("should generate D-Bus blocking API");

    fs::write("./src/secret_service_blocking/api.rs", api)
        .expect("should write generated Secret Service blocking API");
}

#[cfg(feature = "secret-service-nonblock")]
fn generate_nonblock_api(xml: &str, opts: &mut dbus_codegen::GenOpts) {
    opts.connectiontype = dbus_codegen::ConnectionType::Nonblock;

    let api = dbus_codegen::generate(xml, &opts).expect("should generate D-Bus nonblock API");

    fs::write("./src/secret_service_nonblock/api.rs", api)
        .expect("should write generated Secret Service nonblock API");
}
