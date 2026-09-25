# Local model selection

Open **Add transaction → AI** to choose a model. The selection applies to all local profiles on this device; it is independent of financial accounts and login credentials.

| Model | Approximate download | Estimated working RAM |
| --- | ---: | ---: |
| Qwen3 0.6B | 0.37 GiB | 1.5 GiB |
| Qwen3 1.7B | 1.03 GiB | 2.5 GiB |
| Qwen3 4B | 2.33 GiB | 4.5 GiB |
| Qwen3 8B | 4.68 GiB | 7.5 GiB |
| Qwen3 14B | 8.38 GiB | 12 GiB |

All choices are below 32B parameters and use Q4_K_M quantization. The catalog retains the previous Unsloth 0.6B file for existing installations, uses Unsloth for 1.7B, and Qwen's official GGUF releases for 4B, 8B and 14B. Exact revisions, sizes and SHA-256 digests are pinned in `crates/application/src/model_catalog.rs`. Publisher sources: [0.6B](https://huggingface.co/unsloth/Qwen3-0.6B-GGUF), [1.7B](https://huggingface.co/unsloth/Qwen3-1.7B-GGUF), [4B](https://huggingface.co/Qwen/Qwen3-4B-GGUF), [8B](https://huggingface.co/Qwen/Qwen3-8B-GGUF), [14B](https://huggingface.co/Qwen/Qwen3-14B-GGUF).

The app displays total RAM, currently available RAM, free storage and an assessment before installation. These are conservative application estimates for the current 2048-token context and CPU runtime, not measured guarantees of speed or accuracy. Larger models can be slow on CPUs. Mobile operating systems may terminate an app below the device's total RAM limit; start with 0.6B or 1.7B. Unknown hardware measurements are explicitly reported as unknown.

Clearly insufficient total RAM or storage blocks installation. Low available RAM produces a warning; before actually loading weights, the worker checks available RAM again and returns an error instead of intentionally loading a model that exceeds the estimate. Download requests are bounded in size and time, show progress, and can be cancelled. Partial files are temporary. Existing weights and the active selection are retained until the replacement is fully verified and its selection is saved. Switching back to a downloaded model verifies its file before activation.

Models are not included in installers or financial backups. Downloads send only requests for pinned public model files. Financial text stays in the local runtime. Model output still passes through the same domain validation and user confirmation; changing models cannot directly write ledger entries.
