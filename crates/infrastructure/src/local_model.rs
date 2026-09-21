//! Pinned CPU runtime. Each request has a fresh KV cache and no ledger/network access.
use ledger_application::{AppError, ModelOperation, ModelPhase, QuickEntryModel};
use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{AddBos, LlamaModel, params::LlamaModelParams},
    sampling::LlamaSampler,
};
use std::{
    num::NonZeroU32,
    path::Path,
    sync::{OnceLock, atomic::Ordering},
    time::{Duration, Instant},
};

static BACKEND: OnceLock<Result<LlamaBackend, AppError>> = OnceLock::new();
const CONTEXT: u32 = 2048;
const OUTPUT_TOKENS: usize = 512;
const BATCH: usize = 128;

pub struct LocalModel {
    model: LlamaModel,
}

fn runtime_error() -> AppError {
    AppError::Input("AI ในเครื่องอ่านรายการไม่สำเร็จ ลองเขียนให้สั้นลงหรือใช้แบบฟอร์ม".into())
}
fn backend() -> Result<&'static LlamaBackend, AppError> {
    BACKEND
        .get_or_init(|| {
            let mut backend = LlamaBackend::init().map_err(|_| runtime_error())?;
            backend.void_logs();
            Ok(backend)
        })
        .as_ref()
        .map_err(Clone::clone)
}

impl LocalModel {
    /// Call only after the model store has verified the pinned bytes.
    pub(crate) fn load(path: &Path) -> Result<Self, AppError> {
        if !path.is_file() {
            return Err(runtime_error());
        }
        let model = LlamaModel::load_from_file(
            backend()?,
            path,
            &LlamaModelParams::default().with_n_gpu_layers(0),
        )
        .map_err(|_| runtime_error())?;
        Ok(Self { model })
    }
}

impl QuickEntryModel for LocalModel {
    fn propose(&mut self, source: &str, operation: &ModelOperation) -> Result<String, AppError> {
        if source.trim().is_empty() || source.chars().count() > 1000 || source.contains("<|") {
            return Err(AppError::Input(
                "ใช้ข้อความรายการไม่เกิน 1,000 ตัวอักษร ไม่ใส่คำสั่งระบบ".into(),
            ));
        }
        let started = Instant::now();
        let check = || {
            if operation.cancelled.load(Ordering::Relaxed) {
                Err(AppError::Input("ยกเลิกการอ่านรายการแล้ว".into()))
            } else if started.elapsed() > Duration::from_secs(90) {
                Err(AppError::Input(
                    "AI ใช้เวลานานเกินไป ลองข้อความสั้นลงหรือแบบฟอร์ม".into(),
                ))
            } else {
                Ok(())
            }
        };
        check()?;
        operation.set_phase(ModelPhase::Reading);
        let user = serde_json::json!({"user_text": source}).to_string();
        // Qwen3 ChatML, explicitly disable thinking. Pinned model, not a guessed
        // template for arbitrary user-supplied weights. User text is JSON data.
        let prompt = format!(
            "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}\n/no_think<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n",
            include_str!("../../../prompts/quick-entry-runtime.txt"),
            user
        );
        let tokens = self
            .model
            .str_to_token(&prompt, AddBos::Never)
            .map_err(|_| runtime_error())?;
        if tokens.len() + OUTPUT_TOKENS >= CONTEXT as usize {
            return Err(runtime_error());
        }
        let mut context = self
            .model
            .new_context(
                backend()?,
                LlamaContextParams::default()
                    .with_n_ctx(NonZeroU32::new(CONTEXT))
                    .with_n_batch(BATCH as u32)
                    .with_n_ubatch(BATCH as u32)
                    .with_n_threads(4)
                    .with_n_threads_batch(4),
            )
            .map_err(|_| runtime_error())?;
        let mut batch = LlamaBatch::new(BATCH, 1);
        for (chunk_index, chunk) in tokens.chunks(BATCH).enumerate() {
            check()?;
            batch.clear();
            for (i, token) in chunk.iter().enumerate() {
                let position = chunk_index * BATCH + i;
                batch
                    .add(*token, position as i32, &[0], position + 1 == tokens.len())
                    .map_err(|_| runtime_error())?;
            }
            context.decode(&mut batch).map_err(|_| runtime_error())?;
        }
        let grammar = LlamaSampler::grammar(&self.model, &source_grammar(source)?, "root")
            .map_err(|_| runtime_error())?;
        let mut sampler = LlamaSampler::chain_simple([grammar, LlamaSampler::greedy()]);
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut output = String::new();
        operation.set_phase(ModelPhase::Generating);
        for index in 0..OUTPUT_TOKENS {
            check()?;
            let token = sampler.sample(&context, batch.n_tokens() - 1);
            // sample() already accepts the token into the grammar in this pinned
            // wrapper. Accepting twice corrupts the grammar state.
            if self.model.is_eog_token(token) {
                return Ok(output);
            }
            output.push_str(
                &self
                    .model
                    .token_to_piece(token, &mut decoder, false, None)
                    .map_err(|_| runtime_error())?,
            );
            operation
                .generated_tokens
                .store((index + 1) as u64, Ordering::Relaxed);
            // Return only a complete object; truncation never becomes a proposal.
            if serde_json::from_str::<serde_json::Value>(&output).is_ok() {
                return Ok(output);
            }
            if output.len() > 16_384 {
                return Err(runtime_error());
            }
            batch.clear();
            batch
                .add(token, (tokens.len() + index) as i32, &[0], true)
                .map_err(|_| runtime_error())?;
            context.decode(&mut batch).map_err(|_| runtime_error())?;
        }
        Err(runtime_error())
    }
}

/// Small fields with a finite source vocabulary are constrained at sampling time.
/// The application still validates the whole JSON and all free text independently.
fn source_grammar(source: &str) -> Result<String, AppError> {
    let numbers: Vec<&str> = source
        .split(|c: char| !c.is_ascii_digit() && !matches!(c, '.' | ',' | '-' | '+'))
        .filter(|word| word.chars().any(|c| c.is_ascii_digit()))
        .collect();
    let currencies: Vec<&str> = [
        "บาท",
        "฿",
        "THB",
        "thb",
        "USD",
        "usd",
        "EUR",
        "eur",
        "JPY",
        "jpy",
        "ดอลลาร์",
        "ยูโร",
        "เยน",
        "$",
        "€",
        "¥",
    ]
    .into_iter()
    .filter(|word| source.contains(word))
    .collect();
    let mut dates: Vec<&str> = ["วันนี้", "เมื่อวาน"]
        .into_iter()
        .filter(|word| source.contains(word))
        .collect();
    dates.extend(
        numbers
            .iter()
            .copied()
            .filter(|word| word.len() == 10 && word.bytes().filter(|b| *b == b'-').count() == 2),
    );
    let mut grammar = include_str!("../../../prompts/quick-entry.gbnf").to_owned();
    for (name, words) in [
        ("amount", numbers),
        ("currency", currencies),
        ("date", dates),
    ] {
        grammar.push_str(&format!("\n{name} ::= \"null\""));
        for word in words {
            let json = serde_json::to_string(word).map_err(|_| runtime_error())?;
            let terminal = serde_json::to_string(&json).map_err(|_| runtime_error())?;
            grammar.push_str(&format!(" | {terminal}"));
        }
        grammar.push('\n');
    }
    Ok(grammar)
}
