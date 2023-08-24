use cc;

fn main() {
    println!("cargo:warning=BUILDING NOW");

    // Build the ggml and llama libraries from the subproject
    println!("cargo:rerun-if-changed=ext/llama.cpp/ggml.c");
    println!("cargo:rerun-if-changed=ext/llama.cpp/llama.cpp");

    // TODO(aduffy): Build with CUDA support

    cc::Build::new()
        .includes(vec!["ext/llama.cpp"])
        .file("ext/llama.cpp/ggml.c")
        .compile("ggml");

    // Build llama.cpp
    cc::Build::new()
        .cpp(true)
        .flag("-std=c++11")
        .includes(vec!["ext/llama.cpp"])
        .file("ext/llama.cpp/llama.cpp")
        .compile("llama");

    // Create bindings to the ggml models
    bindgen::builder()
        .generate_comments(true)
        .header("ext/llama.cpp/ggml.h")
        .generate()
        .unwrap()
        .write_to_file("src/ggml.rs")
        .unwrap();

    // Make a new bindings file for llama.cpp
}

