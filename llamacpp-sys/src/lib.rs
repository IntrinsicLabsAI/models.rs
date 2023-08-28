#[allow(
    dead_code,
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case
)]
mod llama_bindings;

pub use llama_bindings::{
    llama_backend_free, llama_backend_init, llama_context, llama_context_default_params,
    llama_eval, llama_free, llama_free_model, llama_get_logits, llama_get_timings,
    llama_load_model_from_file, llama_model, llama_n_vocab, llama_new_context_with_model,
    llama_reset_timings, llama_sample_grammar, llama_sample_token, llama_sample_token_greedy,
    llama_sample_top_k, llama_time_us, llama_token, llama_token_bos, llama_token_data,
    llama_token_data_array, llama_token_eos, llama_token_get_text, llama_token_nl, llama_tokenize,
};
