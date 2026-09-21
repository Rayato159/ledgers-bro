# Typography and navigation — 20 September 2026

Removed the footer slogan requested by the user. All five navigation buttons now show their existing, distinct SVG icons on desktop as well as mobile. Their text labels and current-page accessibility attributes remain present.

The application embeds the original TH Sarabun New font in regular, bold, italic and bold italic, with no remote font request or system installation. The [font source, hashes and license](../crates/ui/assets/fonts/README.md) are retained alongside the unmodified font files. The desktop packaging script includes these notices and original font files under `target/release/licenses/THSarabunNew`.

The shared CSS type scale uses 24 px body text and input text, 20 px supporting text, 28–40 px headings, and larger account totals. Sizes use rem units. Mobile navigation uses 20 px labels and 22 px icons; desktop navigation uses 24 px labels. These CSS sizes account for TH Sarabun New's relatively small glyph proportions. Both the base stylesheet and illustration/receipt stylesheet use the same scale, including narrow-screen rules.

## Checks performed

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` passed.
- Debug build and optimized desktop package completed successfully.
- Native Windows WebView reported the embedded regular, bold and italic faces loaded when used, with no font errors. Bold italic is bundled and not requested by the inspected screens.
- On the overview at 1280 px: all five nav icons displayed at 22 px; labels used TH Sarabun New at 24 px; the removed slogan was absent from the DOM; no document horizontal overflow.
- At 390 px: all five nav labels displayed at 20 px without clipping; icons remained visible; supporting text inspected at 20 px; no document horizontal overflow. Visually inspected overview and receipt-entry screens.
- At 360 px: opened the new-account dialog, confirmed all three input/select controls used TH Sarabun New at 24 px without horizontal clipping. Dialog stayed within the viewport. Closed without creating an account.
- At 820 px: all five nav icons and labels remained visible without clipping or horizontal document overflow.

Screenshots are in `.preview/typography-desktop.png`, `.preview/typography-mobile.png`, `.preview/typography-entry-mobile.png` and `.preview/typography-dialog-mobile.png`. These are native Windows WebView layout checks at different CSS viewport widths, not Android device or emulator tests. No financial business logic or database schema changed in this update.
