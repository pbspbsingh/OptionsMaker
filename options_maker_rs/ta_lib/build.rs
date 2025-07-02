use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let talib_src_dir =
        std::env::var("TALIB_SRC_DIR").unwrap_or_else(|_| "./ta-lib-src".to_owned());

    let talib_src_path = PathBuf::from(&talib_src_dir);

    // Build TA-Lib statically
    let mut build = cc::Build::new();

    build
        .flag("-Wno-unused-parameter")
        .flag("-Wno-unused-variable");

    // Core abstract interface
    build.file(
        talib_src_path
            .join("src")
            .join("ta_abstract")
            .join("ta_abstract.c"),
    );
    // add func files
    let func_path = talib_src_path.join("src").join("ta_func");
    for entry in func_path.read_dir()? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name.ends_with(".c") {
            build.file(entry.path());
        }
    }

    // Include directories
    build.include(talib_src_path.join("include"));
    build.include(talib_src_path.join("src").join("ta_abstract"));
    build.include(
        talib_src_path
            .join("src")
            .join("ta_abstract")
            .join("frames"),
    );
    build.include(talib_src_path.join("src").join("ta_common"));
    build.include(talib_src_path.join("src").join("ta_func"));

    // Common source files
    let common_path = talib_src_path.join("src").join("ta_common");
    build.file(common_path.join("ta_version.c"));
    build.file(common_path.join("ta_retcode.c"));
    build.file(common_path.join("ta_global.c"));

    // Compiler flags
    build.flag("-O3");
    build.define("TA_FUNC_NO_RANGE_CHECK", None);

    // Compile the library
    build.compile("ta_lib");

    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header(
            talib_src_path
                .join("include")
                .join("ta_libc.h")
                .to_string_lossy(),
        )
        .header(
            talib_src_path
                .join("include")
                .join("ta_abstract.h")
                .to_string_lossy(),
        )
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()?;

    let out_path = PathBuf::from(std::env::var("OUT_DIR")?);
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", talib_src_dir);

    Ok(())
}
