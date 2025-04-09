fn main() {
    cxx_build::bridge("src/lib.rs")
        .file("predictors/ffi/interface.cpp")
        .std("c++14")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-unused-value")
        .flag("-Wno-switch")
        .compile("cbp-experiments");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=predictors/ffi/interface.cpp");
    println!("cargo:rerun-if-changed=predictors/ffi/interface.h");
}
