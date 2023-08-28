

fn main() {
    // Build the ggml and llama libraries from the subproject
    println!("cargo:rerun-if-changed=ext/llama.cpp/ggml.c");
    println!("cargo:rerun-if-changed=ext/llama.cpp/llama.cpp");

    // Link with macOS Accelerate BLAS implementation
    println!("cargo:rustc-link-lib=framework=Accelerate");

    // Build ggml shared library
    cc::Build::new()
        .cpp(false)
        .includes(vec!["ext/llama.cpp"])
        .opt_level(3)
        .define("GGML_USE_ACCELERATE", "1")
        .file("ext/llama.cpp/ggml.c")
        .file("ext/llama.cpp/ggml-alloc.c")
        .compile("ggml");

    // Build llama.cpp shared library
    cc::Build::new()
        .cpp(true)
        .flag("-std=c++11")
        .includes(vec!["ext/llama.cpp"])
        .opt_level(3)
        .file("ext/llama.cpp/llama.cpp")
        .compile("llama");

    // Create bindings to llama functions
    bindgen::builder()
        .generate_comments(true)
        .clang_args(vec!["-x", "c++", "-std=c++11"])
        .header("ext/llama.cpp/llama.h")
        .generate()
        .unwrap()
        .write_to_file("src/llama_bindings.rs")
        .unwrap();
}

