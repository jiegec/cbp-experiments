fn main() {
    let mut build = cxx_build::bridge("src/lib.rs");

    for file in std::fs::read_dir("predictors/wrapper").unwrap() {
        let file = file.unwrap();
        let path = file.path();
        println!("cargo:rerun-if-changed={}", path.display());
        if path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .ends_with(".cpp")
        {
            build.file(path);
        }
    }

    build
        .std("c++14")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-unused-value")
        .flag("-Wno-switch")
        .flag("-Wno-unused-variable")
        .flag("-Wno-unused-but-set-variable")
        .flag("-Wno-parentheses")
        .compile("cbp-experiments");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");
}
