#![cfg(feature = "llm-cuda")]

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::AddBos;
use llama_cpp_2::model::LlamaModel;
use std::num::NonZeroU32;
use std::path::Path;
use std::time::Instant;

#[test]
fn test_qwen_llama_cuda_gpu_prototype() -> Result<(), Box<dyn std::error::Error>> {
    let model_path_str = r"C:\naina-os\models\qwen-7b-instruct-q4_k_m.gguf";
    let model_path = Path::new(model_path_str);
    assert!(
        model_path.exists(),
        "Qwen GGUF model file not found at {model_path_str}"
    );

    println!("\n========================================================");
    println!("NAINA OS — QWEN DEDICATED CUDA GPU PROTOTYPE BENCHMARK");
    println!("========================================================");

    // Step A: CUDA / Backend Initialization
    let t0 = Instant::now();
    let backend = LlamaBackend::init()?;
    let cuda_init_duration = t0.elapsed();
    println!("A. CUDA Backend Init Time: {:?}", cuda_init_duration);

    // Step B: Load Model into GPU VRAM (n_gpu_layers = 99 offloads all 28 layers + LM head)
    let t_load_start = Instant::now();
    let model_params = LlamaModelParams::default().with_n_gpu_layers(99);
    let model = LlamaModel::load_from_file(&backend, model_path, &model_params)?;
    let model_load_duration = t_load_start.elapsed();
    let cold_start_total = t0.elapsed();
    println!("B. Model Load Time (GPU VRAM): {:?}", model_load_duration);
    println!("C. Cold Start Total:           {:?}", cold_start_total);
    println!("--------------------------------------------------------");

    // Context setup
    let n_ctx = NonZeroU32::new(2048).unwrap();
    let ctx_params = LlamaContextParams::default().with_n_ctx(Some(n_ctx));
    let mut ctx = model.new_context(&backend, ctx_params)?;

    let prompt = "<|im_start|>system\nYou are NAINA OS.<|im_end|>\n<|im_start|>user\nReply with exactly: NAINA ONLINE<|im_end|>\n<|im_start|>assistant\n";

    // Run Warmup
    println!("Running Warmup Run...");
    let warmup_tokens = model.str_to_token(prompt, AddBos::Always)?;
    let mut warmup_batch = LlamaBatch::new(512, 1);
    for (i, &tok) in warmup_tokens.iter().enumerate() {
        let is_last = i == warmup_tokens.len() - 1;
        warmup_batch.add(tok, i as i32, &[0], is_last)?;
    }
    ctx.decode(&mut warmup_batch)?;
    ctx.clear_kv_cache();

    // Benchmark 10 Warm Iterations (16 Tokens generated per iteration)
    println!("\nBenchmark Execution (10 Warm Runs, 16 Tokens Each):");

    let num_warm_runs = 10;
    let target_gen_tokens = 16;
    let mut total_durations = Vec::new();
    let mut ttfts = Vec::new();
    let mut subsequent_avgs = Vec::new();

    for run_idx in 1..=num_warm_runs {
        ctx.clear_kv_cache();
        let run_start = Instant::now();

        // Prompt tokenization
        let prompt_tokens = model.str_to_token(prompt, AddBos::Always)?;
        let prompt_len = prompt_tokens.len();

        let mut batch = LlamaBatch::new(512, 1);
        for (i, &tok) in prompt_tokens.iter().enumerate() {
            let is_last = i == prompt_tokens.len() - 1;
            batch.add(tok, i as i32, &[0], is_last)?;
        }

        // TTFT / First token decode
        let ttft_start = Instant::now();
        ctx.decode(&mut batch)?;
        let ttft = ttft_start.elapsed();

        let mut generated_tokens = Vec::new();
        let mut subsequent_durations = Vec::new();
        let mut current_pos = prompt_len as i32;

        for step in 0..target_gen_tokens {
            let step_start = Instant::now();
            let candidates = ctx.candidates_ith(batch.n_tokens() - 1);
            let next_token = candidates
                .max_by(|a, b| a.logit().partial_cmp(&b.logit()).unwrap())
                .map(|td| td.id())
                .expect("candidates should not be empty");
            generated_tokens.push(next_token);

            let step_elapsed = step_start.elapsed();
            if step > 0 {
                subsequent_durations.push(step_elapsed);
            }

            batch.clear();
            batch.add(next_token, current_pos, &[0], true)?;
            current_pos += 1;
            ctx.decode(&mut batch)?;
        }

        let run_total = run_start.elapsed();

        let sub_avg_ms = if subsequent_durations.is_empty() {
            0.0
        } else {
            subsequent_durations
                .iter()
                .map(|d| d.as_secs_f64() * 1000.0)
                .sum::<f64>()
                / subsequent_durations.len() as f64
        };

        total_durations.push(run_total);
        ttfts.push(ttft);
        subsequent_avgs.push(sub_avg_ms);

        let gen_text = generated_tokens
            .iter()
            .map(|t| {
                model
                    .token_to_str(*t, llama_cpp_2::model::Special::Tokenize)
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join("");

        println!(
            "Run {:2}: Total={:8.3?} | TTFT={:8.3?} | SubAvg={:6.2}ms | Text: {:?}",
            run_idx, run_total, ttft, sub_avg_ms, gen_text
        );
    }

    // Statistical Summary
    total_durations.sort();
    ttfts.sort();
    subsequent_avgs.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mean_total: f64 =
        total_durations.iter().map(|d| d.as_secs_f64()).sum::<f64>() / num_warm_runs as f64;
    let mean_ttft: f64 = ttfts.iter().map(|d| d.as_secs_f64()).sum::<f64>() / num_warm_runs as f64;
    let mean_sub: f64 = subsequent_avgs.iter().sum::<f64>() / num_warm_runs as f64;
    let tok_per_sec = (target_gen_tokens as f64) / mean_total;
    let ms_per_tok = (mean_total * 1000.0) / (target_gen_tokens as f64);

    println!("--------------------------------------------------------");
    println!("PROTOTYPE BENCHMARK REPORT (10 WARM RUNS):");
    println!("C. TTFT / First-Token Latency:");
    println!("   - Min:    {:?}", ttfts[0]);
    println!("   - Max:    {:?}", ttfts[num_warm_runs - 1]);
    println!("   - Mean:   {:.3}ms", mean_ttft * 1000.0);
    println!("   - Median: {:?}", ttfts[num_warm_runs / 2]);
    println!("D. Subsequent Token Avg Latency:");
    println!("   - Mean:   {:.2}ms/token", mean_sub);
    println!(
        "   - Median: {:.2}ms/token",
        subsequent_avgs[num_warm_runs / 2]
    );
    println!("E/F. Total 16-Token Generation Latency:");
    println!("   - Min:    {:?}", total_durations[0]);
    println!("   - Max:    {:?}", total_durations[num_warm_runs - 1]);
    println!("   - Mean:   {:.3}s", mean_total);
    println!("   - Median: {:?}", total_durations[num_warm_runs / 2]);
    println!(
        "Throughput (Tokens/Second):        {:.2} tok/s",
        tok_per_sec
    );
    println!(
        "Per-Token Latency (ms/token):      {:.2} ms/token",
        ms_per_tok
    );
    println!("========================================================\n");

    Ok(())
}
