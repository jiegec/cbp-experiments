fn main() {
    cxx_build::bridge("src/lib.rs")
        .file("predictors/wrapper/interface.cpp")
        .file("predictors/wrapper/andre_seznec_tage_sc_l_8kb.cpp")
        .file("predictors/wrapper/andre_seznec_tage_sc_l_64kb.cpp")
        .file("predictors/wrapper/andre_seznec_tage_sc_l_64kb_only_tage.cpp")
        .std("c++14")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-unused-value")
        .flag("-Wno-switch")
        .compile("cbp-experiments");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=predictors/wrapper/interface.cpp");
    println!("cargo:rerun-if-changed=predictors/wrapper/interface.h");
    println!("cargo:rerun-if-changed=predictors/wrapper/utils.h");
    println!("cargo:rerun-if-changed=predictors/wrapper/andre_seznec_tage_sc_l_8kb.cpp");
    println!("cargo:rerun-if-changed=predictors/wrapper/andre_seznec_tage_sc_l_64kb.cpp");
    println!("cargo:rerun-if-changed=predictors/wrapper/andre_seznec_tage_sc_l_64kb_only_tage.cpp");
}
