use anyhow::{Context, Error, Result};
use std::{
    ffi::{CStr, CString},
    path::{Path, PathBuf},
    ptr::NonNull,
};
use tokio::sync::mpsc::Sender;

use llamacpp_sys::{
    llama_backend_free, llama_backend_init, llama_context, llama_context_default_params,
    llama_eval, llama_free, llama_free_model, llama_get_logits, llama_load_model_from_file,
    llama_model, llama_n_vocab, llama_new_context_with_model, llama_sample_token, llama_token,
    llama_token_bos, llama_token_data, llama_token_data_array, llama_token_eos,
    llama_token_get_text, llama_token_nl, llama_tokenize,
};

pub struct Backend;

impl Backend {
    pub fn new() -> Self {
        unsafe { llama_backend_init(false) };

        Backend
    }

    pub fn load_model(&self, path: &PathBuf) -> Result<Model> {
        Model::new(path)
    }
}

impl Default for Backend {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        unsafe { llama_backend_free() };
    }
}

pub struct Model {
    source: PathBuf,
    ctx: NonNull<llama_context>,
    model: NonNull<llama_model>,
    n_vocab: i32,
    token_bos: llama_token,
    token_eos: llama_token,
    token_nl: llama_token,
}

unsafe impl Send for Model {}
unsafe impl Sync for Model {}

impl Drop for Model {
    fn drop(&mut self) {
        unsafe {
            llama_free_model(self.model.as_mut());
            llama_free(self.ctx.as_mut());
        }
    }
}

impl Model {
    pub fn new(path: &Path) -> Result<Self> {
        let (ctx, model, n_vocab, token_bos, token_eos, token_nl) = unsafe {
            let params = llama_context_default_params();
            let path_c_str = CString::new(path.to_str().expect("Could not convert PathBuf to str"))
                .expect("Could not convert to CString");

            let model = llama_load_model_from_file(path_c_str.as_ptr(), params);

            if model.is_null() {
                return Err(Error::msg("llama_model is NULL"));
            }

            let ctx = llama_new_context_with_model(model, params);
            if ctx.is_null() {
                return Err(Error::msg("llama_context is NULL"));
            }

            let n_vocab = llama_n_vocab(ctx);
            let token_bos = llama_token_bos(ctx);
            let token_eos = llama_token_eos(ctx);
            let token_nl = llama_token_nl(ctx);

            // How to return a Result type here properly
            (
                NonNull::new_unchecked(ctx),
                NonNull::new_unchecked(model),
                n_vocab,
                token_bos,
                token_eos,
                token_nl,
            )
        };

        Ok(Model {
            source: path.to_path_buf(),
            ctx,
            model,
            n_vocab,
            token_bos,
            token_eos,
            token_nl,
        })
    }

    pub fn generate(&mut self, prompt: &str) -> String {
        // Tokenize the prompt, set it, and then run EVAL to get the target outputs
        let mut tokens = [0i32; 4096];
        let prompt_c_str = CString::new(prompt).expect("unable to cast &str to CString");
        let prompt_tokens = unsafe {
            llama_tokenize(
                self.ctx.as_mut(),
                prompt_c_str.as_ptr(),
                tokens.as_mut_ptr(),
                4096,
                false,
            )
        };
        assert!(prompt_tokens > 0, "No tokens generated");

        let mut completion = String::from("");
        for i in 0..20 {
            unsafe {
                assert_eq!(
                    0,
                    llama_eval(self.ctx.as_mut(), tokens.as_ptr(), prompt_tokens + i, i, 4),
                    "llama_eval returned non-zero"
                );

                let logits = llama_get_logits(self.ctx.as_mut());
                let mut candidates: Vec<llama_token_data> =
                    Vec::with_capacity(self.n_vocab as usize);
                for tok_id in 0..self.n_vocab {
                    candidates.push(llama_token_data {
                        id: tok_id,
                        logit: *logits.offset(tok_id as isize),
                        // NOTE(aduffy): We'd set this if we used top-p sampling
                        p: 0.0f32,
                    })
                }
                let mut candidates_array = llama_token_data_array {
                    data: candidates.as_mut_ptr(),
                    size: candidates.len(),
                    sorted: false,
                };

                let next_token = llama_sample_token(self.ctx.as_mut(), &mut candidates_array);
                if next_token == self.token_eos || next_token == self.token_bos {
                    break;
                }
                tokens[(prompt_tokens + i) as usize] = next_token;
                completion.push_str(&self.token_text(next_token));
            }
        }

        completion
    }

    pub async fn generate_stream(&mut self, prompt: &str, channel: Sender<StreamMessage>) {
        let mut tokens = [0i32; 4096];
        let prompt_c_str = CString::new(prompt).expect("unable to cast &str to CString");
        let prompt_tokens = unsafe {
            llama_tokenize(
                self.ctx.as_mut(),
                prompt_c_str.as_ptr(),
                tokens.as_mut_ptr(),
                4096,
                false,
            )
        };
        assert!(prompt_tokens > 0, "No tokens generated");
        for i in 0..20 {
            unsafe {
                assert_eq!(
                    0,
                    llama_eval(self.ctx.as_mut(), tokens.as_ptr(), prompt_tokens + i, i, 4),
                    "llama_eval returned non-zero"
                );

                let logits = llama_get_logits(self.ctx.as_mut());
                let mut candidates: Vec<llama_token_data> =
                    Vec::with_capacity(self.n_vocab as usize);
                for tok_id in 0..self.n_vocab {
                    candidates.push(llama_token_data {
                        id: tok_id,
                        logit: *logits.offset(tok_id as isize),
                        // NOTE(aduffy): We'd set this if we used top-p sampling
                        p: 0.0f32,
                    })
                }
                let mut candidates_array = llama_token_data_array {
                    data: candidates.as_mut_ptr(),
                    size: candidates.len(),
                    sorted: false,
                };

                let next_token = llama_sample_token(self.ctx.as_mut(), &mut candidates_array);
                if next_token == self.token_eos || next_token == self.token_bos {
                    break;
                }
                tokens[(prompt_tokens + i) as usize] = next_token;
                channel
                    .send(StreamMessage::NextToken(self.token_text(next_token)))
                    .await
                    .context("failed to send generated token to receiver")
                    .unwrap();
            }
        }

        channel
            .send(StreamMessage::Done)
            .await
            .context("failed to send Done token to receiver")
            .unwrap();
    }

    // Accept a channel as an argument, and then stream the tokens back over the channel

    fn token_text(&self, token_id: llama_token) -> String {
        let next_token = unsafe { llama_token_get_text(self.ctx.as_ptr(), token_id) };
        if next_token.is_null() {
            panic!("null next_token recovered");
        }
        if token_id == self.token_nl {
            return "\n".to_string();
        }
        let token_text = unsafe {
            CStr::from_ptr(next_token)
                .to_str()
                .expect("Failed to convert to &str")
                .to_string()
        };

        let string = String::from_utf8(vec![0xe2, 0x96, 0x81]).unwrap();
        token_text.replace(&string, " ")
    }
}

pub enum StreamMessage {
    Done,
    NextToken(String),
}
