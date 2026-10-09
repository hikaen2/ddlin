use std::{env, path::PathBuf};

const SOURCES: &[&str] = &[
    "appendix", "appsub", "bcd", "binary", "bitmap", "book", "copyright",
    "cross", "eb", "endword", "error", "exactword", "filename", "font", "hook", "jacode",
    "keyword", "lock", "log", "match", "menu", "multi", "narwalt", "narwfont", "readtext",
    "search", "setword", "stopcode", "strcasecmp", "subbook", "text", "widealt",
    "widefont", "word", "zio",
];

fn main() {
    let vendor = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../vendor/eb");
    let src = vendor.join("eb");

    let mut build = cc::Build::new();
    build
        .include(&vendor)
        .include(&src)
        .define("HAVE_CONFIG_H", None)
        .define("EB_BUILD_LIBRARY", None)
        .warnings(false);
    if env::var("CARGO_CFG_WINDOWS").is_ok() {
        build.define("DOS_FILE_PATH", None).define("WIN32", None).define("getdcwd", "_getdcwd");
    }
    if let Ok(inc) = env::var("DEP_Z_INCLUDE") {
        build.include(inc);
    }
    for name in SOURCES {
        build.file(src.join(format!("{name}.c")));
    }
    build.compile("eb");
    println!("cargo:rerun-if-changed={}", vendor.display());

    bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", vendor.display()))
        .allowlist_function("eb_.*")
        .allowlist_type("EB_.*")
        .allowlist_var("EB_.*")
        .generate()
        .expect("bindgen")
        .write_to_file(PathBuf::from(env::var("OUT_DIR").unwrap()).join("eb.rs"))
        .unwrap();
}
