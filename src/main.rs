use std::env;
use std::io::{self, Write};
use std::num::NonZeroU32;
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use llama_cpp_2::context::params::{KvCacheType, LlamaContextParams};
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_sys_2::LLAMA_FLASH_ATTN_TYPE_ENABLED;

const N_BATCH: u32 = 512;
const DEFAULT_CTX: u32 = 4096;
const DEFAULT_MAX_TOKENS: u32 = 64;
const WEIGHT_BUDGET: u64 = 1024 * 1024 * 1024;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: orin <model.gguf> [prompt]");
        return ExitCode::from(2);
    };
    let rest: Vec<String> = args.collect();
    let prompt = if rest.is_empty() {
        "Hi".to_string()
    } else {
        rest.join(" ")
    };

    match infer(Path::new(&path), &prompt) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}

fn infer(path: &Path, prompt: &str) -> Result<(), String> {
    let backend = LlamaBackend::init().map_err(|err| format!("llama backend init failed: {err:?}"))?;
    println!("supports_gpu_offload={}", backend.supports_gpu_offload());

    let model_params = LlamaModelParams::default().with_n_gpu_layers(999);
    let model = LlamaModel::load_from_file(&backend, path, &model_params)
        .map_err(|err| format!("load {}: {err}", path.display()))?;
    let n_layer = model.n_layer();
    let n_params = model.n_params();
    if n_layer != 14 || !(150_000_000..=300_000_000).contains(&n_params) {
        return Err(format!(
            "{} is not LFM2.5-230M (layers {n_layer}, params {n_params})",
            path.display()
        ));
    }
    let weight_bytes = model.size();
    if weight_bytes > WEIGHT_BUDGET {
        return Err(format!(
            "language model is {weight_bytes} bytes and its cap is {WEIGHT_BUDGET}"
        ));
    }

    let n_ctx = env::var("GGMLRS_LLM_CTX")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(DEFAULT_CTX);
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(n_ctx))
        .with_n_batch(N_BATCH)
        .with_n_ubatch(N_BATCH)
        .with_flash_attention_policy(LLAMA_FLASH_ATTN_TYPE_ENABLED)
        .with_offload_kqv(true)
        .with_type_k(KvCacheType::F16)
        .with_type_v(KvCacheType::F16);
    let mut ctx = model
        .new_context(&backend, ctx_params)
        .map_err(|err| format!("context: {err}"))?;
    println!(
        "loaded {} layers={n_layer} params={n_params} n_ctx={} n_batch={N_BATCH} flash_attn=enabled kv=f16 weight_bytes={weight_bytes}",
        path.display(),
        ctx.n_ctx(),
    );

    let messages = [("user".to_string(), prompt.to_string())];
    let chat = prompt_from_messages(&messages)?;
    generate(&model, &mut ctx, &chat, max_tokens())
}

fn max_tokens() -> u32 {
    env::var("ORIN_MAX_TOKENS")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(DEFAULT_MAX_TOKENS)
}

fn temperature() -> f32 {
    env::var("ORIN_TEMPERATURE")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(0.0)
}

/// ChatML with a BOS prefix. The GGUF template is Jinja, and
/// llama_chat_apply_template aborts when it is given that string.
fn prompt_from_messages(messages: &[(String, String)]) -> Result<String, String> {
    if messages.is_empty() {
        return Err("no messages".to_string());
    }
    let mut prompt = String::from("<|startoftext|>");
    for (role, content) in messages {
        if role != "system" && role != "user" && role != "assistant" {
            return Err(format!("unknown role {role}"));
        }
        prompt.push_str("<|im_start|>");
        prompt.push_str(role);
        prompt.push('\n');
        prompt.push_str(content);
        prompt.push_str("<|im_end|>\n");
    }
    prompt.push_str("<|im_start|>assistant\n");
    Ok(prompt)
}

fn generate(
    model: &LlamaModel,
    ctx: &mut LlamaContext<'_>,
    prompt: &str,
    limit_tokens: u32,
) -> Result<(), String> {
    let tokens = model
        .str_to_token(prompt, AddBos::Never)
        .map_err(|err| format!("tokenize: {err}"))?;
    if tokens.is_empty() {
        return Err("prompt produced no tokens".to_string());
    }
    let n_ctx = ctx.n_ctx();
    if tokens.len() >= n_ctx as usize {
        return Err(format!(
            "prompt is {} tokens and the context is {n_ctx}",
            tokens.len()
        ));
    }

    ctx.clear_kv_cache();
    let mut batch = LlamaBatch::new(ctx.n_batch() as usize, 1);
    let prefill_started = Instant::now();
    let mut cursor = 0;
    while cursor < tokens.len() {
        let end = (cursor + ctx.n_batch() as usize).min(tokens.len());
        batch.clear();
        for (offset, token) in tokens[cursor..end].iter().enumerate() {
            let pos = i32::try_from(cursor + offset).map_err(|_| "position overflow".to_string())?;
            let logits = cursor + offset + 1 == tokens.len();
            batch
                .add(*token, pos, &[0], logits)
                .map_err(|err| format!("prefill batch: {err}"))?;
        }
        ctx.decode(&mut batch)
            .map_err(|err| format!("prefill: {err}"))?;
        cursor = end;
    }
    let prefill_secs = prefill_started.elapsed().as_secs_f64().max(1e-9);
    let prefill_tps = tokens.len() as f64 / prefill_secs;

    let temperature = temperature();
    let mut sampler = if temperature <= 0.0 {
        LlamaSampler::greedy()
    } else {
        LlamaSampler::chain(
            [LlamaSampler::temp(temperature), LlamaSampler::dist(0)],
            false,
        )
    };
    let mut decoder = encoding_rs::UTF_8.new_decoder();
    let mut n_decoded = 0u32;
    let room = n_ctx.saturating_sub(tokens.len() as u32);
    let limit = limit_tokens.min(room);
    let decode_started = Instant::now();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    loop {
        if n_decoded >= limit {
            break;
        }
        let token = sampler.sample(ctx, -1);
        sampler.accept(token);
        if model.is_eog_token(token) {
            break;
        }
        let piece = model
            .token_to_piece(token, &mut decoder, false, None)
            .map_err(|err| format!("detokenize: {err}"))?;
        write!(out, "{piece}").map_err(|err| format!("write: {err}"))?;
        out.flush().map_err(|err| format!("write: {err}"))?;
        let pos = i32::try_from(tokens.len() + n_decoded as usize)
            .map_err(|_| "position overflow".to_string())?;
        batch.clear();
        batch
            .add(token, pos, &[0], true)
            .map_err(|err| format!("decode batch: {err}"))?;
        ctx.decode(&mut batch)
            .map_err(|err| format!("decode: {err}"))?;
        n_decoded += 1;
    }
    let decode_secs = decode_started.elapsed().as_secs_f64().max(1e-9);
    let decode_tps = n_decoded as f64 / decode_secs;
    writeln!(out).map_err(|err| format!("write: {err}"))?;
    writeln!(
        out,
        "prefill_tps={prefill_tps} decode_tps={decode_tps} decoded={n_decoded}"
    )
    .map_err(|err| format!("write: {err}"))?;
    Ok(())
}
