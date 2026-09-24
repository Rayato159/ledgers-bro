# Receipt recognition: desktop and Android

The camera button lives inside the quick-entry composer; files can also be dropped onto that composer. Select up to eight JPG/PNG/HEIC/HEIF images, at most 32 MiB per image and 128 MiB per selection. Each image is read sequentially and has its own preview/error. The separate scanner card has been removed. The manual-entry page has the same camera button and immediately opens editable receipt drafts, preserving an existing nonempty entry.

Quick entry appends a clearly delimited block per successful receipt to the existing text. Each block includes total, date, account, category, note, and quoted item/discount/tax lines. Edit these values, then choose **อ่านรายการ**. Unknown amounts stay blank; unclear tax treatment stays **ตรวจวิธีคิด**. A deterministic application parser handles these blocks without an LLM, preserves ordinary income/expense/transfer text around them, and rejects malformed or oversized batches as a whole. Other management commands must be submitted separately from receipt blocks. Eight total entries (including typed transactions) can be reviewed in one batch.

Hosts normalize orientation and strip EXIF/GPS by re-encoding the preview as JPEG. OCR reads those exact pixels. Windows/macOS use `ReceiptImageNormalizer` and `TesseractOcr`; Android uses its native multiple-document picker, ImageDecoder and Tesseract4Android. Desktop uses a native multi-file dialog; the shared HTML input remains available for hosts using file events. All adapters produce untrusted OCR only. The shared receipt editor checks item reconciliation and explicit review for every receipt before atomic batch confirmation. No upload automatically writes a transaction.

## Run

On Windows, install Tesseract with Thai and English language data from the distribution linked below. Pass its installation directory with `--ocr-dir`. Keep its DLL dependencies and `tessdata` beside the executable. On Linux, install Tesseract and the Thai/English language packages; provide a runtime directory containing a `tesseract` link and `tessdata`.

On macOS, install `brew install tesseract`, then run `sh scripts/setup-ocr-macos.sh`. This links the installed reader and verifies the same pinned Thai/English models. JPG/PNG and JPEG EXIF orientation are supported; the current HEIC helper is Windows-specific, so convert HEIC to JPG/PNG for the macOS preview.

Debug desktop builds look in `.tools/ocr` by default. For a packaged executable, ship the runtime and its dependencies/license files in an `ocr` folder next to the executable, or pass `--ocr-dir <absolute-path>`. Missing runtime is a recoverable UI error, never a silent cloud fallback. No receipt is uploaded or used for model training.

## Scope and guardrails

- JPG/PNG/HEIC/HEIF; 32 MiB compressed, 10,000 px per side and 50 megapixels maximum. Container signatures, not filenames, determine format. JPEG EXIF and HEIF container rotation are applied. Live Photo movies, RAW/DNG, AVIF and HEIF sequences are not supported.
- Windows uses pinned ImageMagick 7.1.2-31 Q8 x64 with libheif for HEIF only. A restricted coder policy, source-dimension preflight, no delegates, bounded temporary output and a shared 45-second conversion deadline contain the helper. It downsizes to 4,000 px on the longest side; JPEG/PNG decoding also has a 256 MiB allocation limit. Some extreme images within the nominal pixel limit can still exceed decoder resources and return a recoverable error.
- Android 9+ uses software ImageDecoder, sRGB, at most 3,200 px on the longest side, followed by bundled Tesseract4Android 4.9.0 and pinned `tha+eng` models. SAF grants only the selected document URIs; no broad storage permission. Older Android versions show an explanatory error. Devices still require HEIF codec validation; emulator success is not a claim about every OEM.
- The subprocess is launched directly with fixed arguments, no command shell and no URL input. Timeout is 45 seconds after decode; cancellation kills/waits for the child. Recognition is off the UI/ledger worker thread.
- Working files are in a per-call temporary directory, removed on normal exit/cancellation. Process termination/power loss can leave OS temporary files; this is not a secure-erasure guarantee.
- Thai/ASCII digits, unambiguous four-digit CE/BE dates, and labelled totals. Conflicting totals/dates become choices. New entries default to the supplied local day; when the receipt date is missing or ambiguous, the draft keeps today and the UI labels this as a default. A single recognized receipt date is preserved.
- Foreign-currency markers suppress THB autofill for both payment and line amounts; user must enter actual THB values. Currency detection is conservative, not universal, and this is not currency conversion or tax/VAT calculation.
- OCR text is plain text, never code, prompt instructions or HTML. All values still pass ledger validation and explicit confirmation.
- Receipt image and OCR text are held for review only and cleared on successful commit/new draft. They are **not durable attachments**, evidence for tax filing, or backup.
- Clean printed receipts are the initial target. This is not an accuracy guarantee for handwriting, faded thermal paper, rotated text inside an image, multi-column receipts or every merchant format. Missing/uncertain fields can be entered manually.

## Item details and reconciliation

Named lines ending in a readable amount become editable line candidates. Metadata, subtotals, grand totals, cash tendered and change are excluded from item sums. Repeated purchases are retained. Quantities/unit-price text stays with the description; the trailing amount is the line total. Wrapped/multi-column lines are not reliably reconstructed and require manual correction.

VAT and service-charge lines with unclear treatment require an explicit choice between added and already-included amounts. Discounts subtract, included charges do not add again, and explicitly entered rounding may be positive or negative. The app never invents an adjustment to make OCR numbers balance, or calculates tax liability from these lines.

`ReceiptLine` and `ReceiptBreakdown` are domain value objects. Construction checks exact integer-satang equality between line contributions and the chosen net payment. A difference of even one satang blocks preview. The application checks reconciliation and explicit review again before creating a prepared ledger entry; this is not solely a disabled UI button. Changing line details or the net payment clears the review checkbox.

The confirmed breakdown generates one bullet per line and the net total in the durable journal note, plus the optional user note. History and confirmation preserve line breaks; CSV retains the complete note. Up to 100 lines with descriptions up to 120 characters are accepted. Stored notes are bounded at 20,000 characters to retain full receipt details; no detail is silently truncated. Image/raw OCR text remain temporary rather than durable attachments.

Equality verifies the entered numbers, not OCR's fidelity to the physical receipt. Users must compare each line and the net payment with the image before confirmation.
- Current capture uses a file/document picker on desktop and Android. In-app camera capture and the iOS host are still pending.

Android selection/OCR runs outside the ledger worker. Rust polls volatile native results from a JVM-attached worker, avoiding repeated main-thread dispatch while the document picker backgrounds the Activity. Session IDs reject stale callbacks; cancellation, backgrounding during recognition and deadlines release the UI. Native OCR uses the interruptible recognition entry point before extracting text. Models are verified and copied from APK assets into private no-backup storage; no runtime model download is needed for OCR. Input/preview/text remain transient memory. The recognizer deadline is 45 seconds, with a eight-minute batch result deadline and a five-minute picker deadline on the Rust bridge.

## Source and verification

Runtime: [Tesseract 5.5.3 Windows release](https://github.com/tesseract-ocr/tesseract/releases/tag/5.5.3), linked by [UB Mannheim](https://github.com/UB-Mannheim/tesseract/wiki). Models: [tessdata_fast](https://github.com/tesseract-ocr/tessdata_fast/tree/87416418657359cb625c412a48b6e1d6d41c29bd), `tha` and `eng`, LSTM `--oem 1`. These are an offline desktop baseline, not a claim of best Thai mobile OCR. The adapter can be replaced after device benchmarks. Check redistribution notices for the complete runtime and bundled native dependencies before packaging for sale.

Automated rules tests cover total/subtotal/tax/change separation, exact satang, Thai digits, BE dates, ambiguity, invalid/future dates, foreign currency and oversized input. `tests/fixtures/receipt-th-en.png` is synthetic (not an actual tax invoice). Run the real local reader test explicitly:

```sh
cargo test -p ledger-infrastructure --test receipt_ocr -- --ignored
```

Also run `cargo test -p ledger-infrastructure --test receipt_images -- --include-ignored` for real HEIF decoding, EXIF/container orientation, 24 MP normalization and Thai/English total recognition. Runtime-dependent integration tests are marked ignored in the portable suite and must be invoked explicitly. Fixture provenance and regeneration are in `tests/fixtures/README.md`; these images are synthetic, not captures from a physical iPhone. See [iPhone import instructions](iphone-images-th.md).


## Multi-image verification (2026-09-24)

The portable suite covers formatter/parser boundaries, mixed typed text, duplicate/ambiguous/foreign values, escaped OCR delimiters, exact reconciliation, atomic saving and idempotent retry. UI event tests send multiple files through the real composer drop handler and verify both blocks appear, while upload tests cover a bad image among valid images, cancellation, size limits, and preservation of existing manual input. Long receipt prompts bypass the model worker's ordinary 1,000-character limit and never fall back to AI.

Native macOS verification used synthetic PNG and EXIF-rotated JPEG fixtures plus an invalid PNG: two receipts filled the prompt, the failed image retained its own error, and a preexisting typed transaction remained. The three-entry batch was reviewed and committed with both detailed receipt notes. Manual-page selection also immediately displayed two editable receipt entries without changing pages. The real Thai/English Tesseract integration test passed. Android multiple-selection/JNI changes still require a device/SDK build; a macOS workspace build does not validate the Android host. Real desktop HEIF integration remains a separate runtime-dependent test.

Receipt extraction currently supports THB ledgers only. Other ledger currencies use manual entry; uploads and receipt-formatted prompts are rejected before any draft can reinterpret THB amounts.
