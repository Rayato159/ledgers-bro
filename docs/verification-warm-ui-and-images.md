# Warm UI, cancellable AI and receipt images — 21 September 2026

Scope: Windows desktop and Pixel_6 API 37 x86_64 Android emulator. Used synthetic receipt fixtures and an existing test cash account with THB 1,000. No new transaction was committed during this verification. This is a debug test build, not a Store release.

## UI

- Cream, warm yellow and brown palette applied to shared UI, category SVGs, chart, dialogs and native Android bars. TH Sarabun New and tanuki doodles retained.
- Mobile quick entry moved to the center bottom-navigation action. Overview illustration uses soft pastel shapes and a cream balance card. Desktop keeps a responsive dashboard.
- Typing now appears before receipt import; model configuration is an expandable section.
- Inspected real WebViews on Android and Windows, including overview and Android receipt confirmation. No horizontal overflow in the tested Android viewport. Local screenshots: `.preview/warm-android-overview.png`, `.preview/warm-desktop-overview.png`, `.preview/warm-android-receipt.png`.
- Full accessibility review and all device/font scale combinations were not performed.

## AI responsiveness

- Runtime publishes real verifying/loading/reading/generating phases. UI displays elapsed time and a cancellable operation; no fabricated percent.
- End-to-end UI deadline is 90 seconds, including queue and startup. Cancellation releases the form without awaiting completion of a native batch. Cooperative cancellation still controls the model worker; late output cannot create or replace a draft.
- Tests cover a worker that never replies, explicit cancellation, and late output. A completed future takes priority when it and the deadline are both ready after suspension.
- On the emulator, `น้ำ 10 บาท เงินสด` showed the reading phase and elapsed time. Cancellation re-enabled the input, removed progress, displayed the cancellation message and produced no saved entry. Backgrounding during another run resulted in a recoverable timeout with an enabled form after resume.
- These checks establish responsiveness/cancellation, not a new latency or language-accuracy benchmark. Cold inference can still take significant time on an emulator. Debug builds now optimize SHA-256 verification of the 397 MB model.

## Images and actual OCR

| Check | Windows | Android emulator |
| --- | --- | --- |
| Synthetic JPG/PNG receipt | Local reader passed | JPEG passed |
| HEIF/HEVC receipt, 1000×1200 | Passed | Passed |
| JPEG EXIF orientation 6 | Upright 1000×1200 | Upright 1000×1200 |
| HEIF container rotation | Upright 1000×1200 | Upright 1000×1200 |
| HEIF 4000×6000 | Normalized to max 4000 px | Decoder rejected this fixture; recoverable error |
| Offline Thai/English HEIF OCR | Local process | Passed with Wi-Fi and mobile data both disabled |
| Repeated selections | Automated reader calls | HEIC followed by JPEG passed |
| Cancel document picker | Not re-exercised in this run | Returned to existing THB 160.50 draft; form usable |
| Recovery after decoder error | Unit test covers corrupt input | Successful normal HEIC selection after failed large image |

The ordinary receipt produced Coffee 80.00, Lunch 70.00, VAT 10.50 and net THB 160.50. Cash 200.00 and change 39.50 were not included in the item sum. Android confirmation displayed Thai bullet details. Selecting VAT as added tax reconciled the total; changing payment to 160.51 disabled review and preview. Restoring 160.50 allowed review/preview, and **Save was not pressed**. Network settings were restored to their original enabled values.

No source fixture came from a physical iPhone. HEIF support depends on its encoding and the Android codec. Real iPhone HDR, gain/depth auxiliary images, 10-bit/48 MP variants, physical ARM64 devices and in-app camera capture remain unverified. For rejected variants use JPG/PNG; see [import instructions](iphone-images-th.md). OCR correctness for every real receipt is not established by a matching synthetic total.

## Build and automated evidence

- Workspace formatting, Clippy `-D warnings`, and tests: **93 passed**, 2 runtime-dependent tests skipped by the portable suite.
- Explicit `receipt_images` suite with runtime installed: **3 passed**, including actual ImageMagick/libheif conversion and Thai/English Tesseract reading. The additional ignored HEIF integration test was therefore exercised.
- Android-specific x86_64/mobile Clippy with NDK 30: passed `-D warnings`.
- Windows executable and Android x86_64 debug APK built. APK installed using replacement, without uninstalling/wiping the ledger or model.
- Native Android OCR uses ImageDecoder and Tesseract4Android 4.9.0, not Windows DLLs. Model and runtime hashes are pinned in setup scripts. License notices accompany the build inputs.

APK: `target/android/ledgers-bro-test-x86_64.apk`. The current artifact hash is recorded alongside it in `target/android/ledgers-bro-test-x86_64.sha256` after final build.
