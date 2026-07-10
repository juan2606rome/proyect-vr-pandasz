use std::env;
use std::path::PathBuf;

fn main() {
    let ndk_home = env::var("ANDROID_NDK_HOME").expect("ANDROID_NDK_HOME no está definida");

    // Windows -> "windows-x86_64". Si compilas desde Linux/Mac cambia esto
    // a "linux-x86_64" o "darwin-x86_64" respectivamente.
    let host_tag = "windows-x86_64";
    let sysroot = format!(
        "{}/toolchains/llvm/prebuilt/{}/sysroot",
        ndk_home, host_tag
    );
    let general_include = format!("{}/usr/include", sysroot);
    // target-specific (limits.h, etc. dependen del target concreto)
    let target_include = format!("{}/usr/include/aarch64-linux-android", sysroot);

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", general_include))
        .clang_arg(format!("-I{}", target_include))
        .clang_arg("--target=aarch64-linux-android26")
        .allowlist_function("ACamera.*")
        .allowlist_function("ACaptureRequest.*")
        .allowlist_function("ACaptureSessionOutput.*")
        .allowlist_function("AMediaCodec_.*")
        .allowlist_function("AMediaFormat_.*")
        .allowlist_type("ACamera.*")
        .allowlist_type("ACapture.*")
        .allowlist_type("AMediaCodec.*")
        .allowlist_type("AMediaFormat")
        .allowlist_var("ACAMERA_.*")
        .allowlist_var("AMEDIACODEC_.*")
        .generate()
        .expect("fallo generando bindings de cámara/mediacodec NDK");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("camera_bindings.rs"))
        .expect("no se pudo escribir camera_bindings.rs");

    println!("cargo:rustc-link-lib=camera2ndk");
    println!("cargo:rustc-link-lib=mediandk");
}