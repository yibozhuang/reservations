use std::env;

fn main() {
    let include_path = if let Ok(path) = env::var("PROTOC_INCLUDE") {
        path
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "/opt/homebrew/include".to_string()
        } else {
            "/usr/local/include".to_string()
        }
    } else {
        "/usr/include".to_string()
    };

    tonic_build::configure()
        .compile(
            &["proto/reservations.proto"],
            &[include_path.as_str(), "proto"],
        )
        .expect("Failed to compile proto files");
}
