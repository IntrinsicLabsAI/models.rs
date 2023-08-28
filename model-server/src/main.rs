use std::path::PathBuf;

use anyhow::Result;
use llamacpp::{Backend};

#[tokio::main]
async fn main() -> Result<()> {
    let backend = Backend::new();
    let mut model = backend.load_model(&PathBuf::from("/Users/aduffy/Documents/llama2_gguf.bin"))?;
    println!("LLM Output: {}", model.generate("This is the sound a donkey makes:"));

    Ok(())
}
