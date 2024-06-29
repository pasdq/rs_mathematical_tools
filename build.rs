use std::env;
use std::process::Command;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        // Only run this part if targeting Windows
        let output = Command::new("x86_64-w64-mingw32-windres")
            .args(&["app.rc", "-O", "coff", "-o", "app.res"])
            .output()
            .expect("Failed to run windres");

        if !output.status.success() {
            panic!(
                "windres failed with output: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Link the resource file to the final executable
        println!("cargo:rustc-link-arg=app.res");
    }
}
