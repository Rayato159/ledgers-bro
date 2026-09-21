# Receipt lines and default date — 20 September 2026

## Delivered behavior

- Manual and quick-entry drafts retain their existing default of today's local date. Receipt drafts now also default to today when OCR finds no unique usable date; the UI explicitly identifies this as a default. A unique historical receipt date is preserved.
- Receipt item candidates appear as editable bullet rows. User choices distinguish extra VAT/service charges from amounts already included in prices. Discounts subtract; explicit rounding may add or subtract. No balancing adjustment is fabricated.
- The application checks exact satang equality and user review before preparing a receipt entry. Editing any line or the chosen net amount clears review. The confirmed entry stores the full bullet breakdown and total in its note, retained by history and CSV.
- Chat currently uses a deterministic command parser. There is no LLM runtime connected. Receipt recognition uses the local Tesseract Thai/English adapter, followed by deterministic extraction and reconciliation.

## Verification

- Workspace tests: **54 passed**; the environment-dependent native OCR test remains excluded from the default suite.
- Native OCR test explicitly run: **1 passed**. Real recognition of the synthetic Thai/English image produced Coffee 80.00, Lunch 70.00 and VAT 10.50; choosing added VAT reconciled the lines to 160.50.
- Domain tests cover one-satang mismatches, discounts, extra/included charges, signed rounding, invalid/oversized input, overflow and preservation of 100 bullet lines without truncation.
- Application tests cover date fallbacks, preserving recognized historical dates, duplicate item lines, quantity text, unresolved VAT, missing items, foreign-currency line amounts and explicit review.
- SQLite integration test rejects mismatched/unreviewed previews, commits the verified receipt, reopens the file, checks the complete bullet note and 839.50 balance from a 1,000.00 opening balance, and verifies multiline CSV output.
- Native WebView walkthrough in `.data/receipt-lines-review`: created the synthetic 1,000.00 account; new form date was 2026-09-20; real OCR populated three lines; explicit added-VAT choice and review enabled preview. Changing Coffee from 80.00 to 80.01 cleared review and disabled preview, displaying the 160.51 versus 160.50 mismatch. Restoring the amount allowed preview. Final UI flow committed the receipt and displayed identical bullets in history; overview showed assets 839.50 and expenses 160.50. No browser runtime errors were reported.
- Receipt editor and confirmation inspected at 390 CSS pixels with no horizontal document overflow; saved history inspected at 1280 pixels. This is Windows WebView layout verification, not Android certification.
- Formatting, workspace Clippy with warnings denied, debug build and optimized desktop packaging passed.

The UI walkthrough supplied a synthetic file-selection event using Dioxus Desktop's native-picker full-path representation; the Windows picker dialog itself was not automated. The fixture went through the real image decoder, Tesseract process, parser, application and SQLite path. No OCR text or model output was fabricated.

Screenshots: `.preview/receipt-lines-mobile.png`, `.preview/receipt-lines-confirmation.png` and `.preview/receipt-lines-saved.png`. The confirmation screenshot was captured before the final left-alignment polish; saved history was captured from the final build.

Arithmetic equality is guaranteed for the accepted line values, not for OCR fidelity to the physical receipt. Wrapped lines, unusual merchant layouts and misrecognized text still require user correction against the source image. See [receipt-ocr.md](receipt-ocr.md) for supported inputs and limitations.
