# Local LLM verification — 21 September 2026

Scope: real Qwen3 0.6B Q4_K_M inference through pinned llama-cpp-2 0.1.156, shared Dioxus UI, Windows and Android x86_64 emulator. Financial data used was synthetic. No microphone capture or cloud inference was used.

## Automated checks

- Workspace formatting and Clippy with `-D warnings`: passed. Separate Android x86_64 mobile-feature Clippy check also passed with `-D warnings`.
- Full workspace suite: 87 passed, 1 existing optional OCR integration test ignored. One additional omitted-currency/date regression test was then added; targeted model-resolution suite passed all 9 tests (88 unique passing tests across these runs).
- Checks include monetary substring boundaries, unknown/future dates, invented account/category labels, explicit negation/scope guards, VAT/withholding exclusion, foreign currency even when omitted by the model, multiple alternative drafts, missing/corrupt weights and pre-cancelled installation.
- Integration test verifies model interpretation + preview leave SQLite unchanged; only explicit commit writes an expense, and retrying the same submission does not duplicate it.
- Windows debug executable and Android x86_64 debug APK built successfully. Windows native build needs Visual Studio C++ SDK, CMake and libclang. Android NDK 30 needs an API-qualified bindgen target and the SDK's Ninja generator; the build script now scopes/restores those settings.

## Real-model evaluation

Ran `local_model_probe` against the actual pinned 396,705,472-byte GGUF, not a fake model adapter. Evaluation output is local in `.preview/llm-evaluation.log`; fixtures are synthetic.

Ordinary examples tested: coffee yesterday, misspelled expense verb, salary, own-account transfer, lunch, wages and a misspelled account name. Final amount/intent extraction was usable on these examples. Raw model text was NOT universally correct:

- `ได้ค่าจ้าง 2500 บาทเข้า ธนาคาร` returned the unmentioned category `เงินเดือน`.
- `กาเเฟวันนี้ 65 บาทจ่ายจากเงนสด` normalized the unmentioned account spelling to `เงินสด`.
- A two-purchase request returned a fabricated description/account.

The application now discards ungrounded account/category/description fields and requires selection, without altering amount/currency/date facts. Multiple-write and negated requests are rejected by conservative application scope checks independently of the model. Strict JSON/grounding validation still rejects invented financial facts and extra authority fields. This is NOT a benchmark proving general Thai accuracy; there can still be semantic mistakes that pass structural validation.

Observed desktop inference on this small set was approximately 4–5 seconds for ordinary warm requests and 13.6 seconds for the first cold request, including model verification/loading. These are development observations on this computer, not a mobile latency guarantee.

## UI checks

Windows isolated ledger `.data/llm-review`:

- Created synthetic cash opening balance 1,000 baht.
- Entered `จายค่าอาหาร 120 บาทจากเงินสด`.
- LLM produced a 120-baht choice against cash; category remained unselected.
- Chose the draft, selected food, inspected the final confirmation, then explicitly saved.
- Dashboard showed balance 880 and expenses 120. No write occurred at the model-choice stage.

Android Pixel_6 emulator, API 37, x86_64:

- Updated the test package using install `-r`; no uninstall/data wipe.
- Downloaded the model through the actual in-app HTTPS downloader and passed pinned SHA-256 verification.
- Created a synthetic cash account with 1,000 baht.
- Disabled emulator Wi-Fi and mobile data (both states verified 0). `เมื่อวานซื้อกาแฟ 80 บาท จ่ายเงินสด` still produced an 80-baht choice, cash account, and 2026-09-20 date.
- No expense was committed in the emulator. Starting another inference and leaving the chat cancelled it; overview remained usable without a transaction write.
- Restored both emulator network settings to their original enabled state.
- Updated to the final APK and verified the cash account and downloaded model remained available. `กาเเฟวันนี้ 65 บาทจ่ายจากเงนสด` produced a 65-baht choice with the account left blank and an explicit explanation; it did not auto-bind the model's corrected `เงินสด` spelling. Screenshot: `.preview/llm-android-review.png`.

The APK is a debug-signed test artifact, not a Play Store bundle or production release:

`target/android/ledgers-bro-test-x86_64.apk`

SHA-256: `32F7FF744A9D7DDE156A4C0DCA268BFDE4F0579E72178768678B78CA9E865836`

## Not established by these checks

Physical Android/ARM64 performance, thermal/battery behavior, low-memory robustness across devices, iOS, wide Thai-language accuracy, all download/network failure modes, and Store suitability remain unverified. Native memory allocation failure may terminate the app; cancellation is cooperative between native calls. Existing encryption, backup, biometric, mobile OCR and tax-engine gaps remain as documented in README.
