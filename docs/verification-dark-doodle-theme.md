# Dark doodle UI verification — 21 September 2026

## Scope and deliverables

Presentation-only redesign: dark slate surfaces, white/gray action accents, rounded cards and floating mobile navigation, a light receipt-style confirmation panel, grayscale category SVGs and two newly generated doodle tanukis. TH Sarabun New remains bundled. Both native hosts now use the same `ArtAssets::bundled()` assets. The former room and human mascot images have no references in active host/UI Rust sources.

The website palette was inspected live, then its gold accent was replaced by white/gray as the user requested. The user's later doodle instruction supersedes the earlier 3D draft. Final artwork and complete prompts: [design/dark-doodle-theme.md](../design/dark-doodle-theme.md).

- `crates/ui/assets/tanuki/tanuki-ledger.png`: 1254×1254 RGBA PNG, 976,947 bytes.
- `crates/ui/assets/tanuki/tanuki-phone.png`: 1254×1254 RGBA PNG, 925,554 bytes.
- Both PNGs were visually inspected and their transparent corner alpha verified as zero.
- Final emulator APK: `target/android/ledgers-bro-test-x86_64.apk`, 84,200,415 bytes.
- APK SHA-256: `79CBEE299BFA1D624C7816372AD15E3433D99C3D3356207AF6274F2417EBDF73`.

## Verification performed

| Check | Result |
|---|---|
| Windows debug build | Passed; `.preview/dark-ui-desktop-build.log` |
| Android x86_64 build/install | Passed; `.preview/dark-ui-android-build.log`; same test package and signing, data retained |
| `cargo fmt --all -- --check` | Passed |
| Workspace Clippy, all targets, warnings denied | Passed after final source edits; `.preview/dark-ui-clippy.log` |
| Android x86_64 Clippy, warnings denied | Passed after doodle integration, before final CSS hint-color and removal of decorative English text; `.preview/dark-ui-android-clippy.log` |
| Workspace tests | 67 passed, 1 optional native OCR integration test ignored; `.preview/dark-ui-tests.log`; ran during redesign before final asset/CSS-only refinements |
| Windows native UI via WebView2 | 1280×950 screenshot inspected; isolated `.data/dark-theme-review` account creation and expense preview/confirmation succeeded |
| Accounting smoke through UI | Synthetic opening 1,000 THB − food expense 80 THB = 920 THB; no accounting logic changed |
| Narrow layout | At 360px WebView width, document content width was 345px including scrollbar reservation; no horizontal overflow. Screenshot capture at this emulated viewport timed out, so this is a DOM check only |
| Android native UI | Pixel 6 / Android 17 API 37 / x86_64 emulator; overview screenshot inspected; 411px CSS viewport and 411px document width |
| Mobile navigation | Overview, accounts, transaction list, quick entry and tax status opened; measured overview/accounts/list/quick entry/tax screens without horizontal overflow |
| Account dialog | Dark surface and inputs verified; dialog fits the available mobile viewport; closed without creating a record |
| Quick-entry preview | Existing Android test account resolved from a Thai command; amount 80 THB and Food category populated; receipt-style preview opened without committing a new transaction |
| Final contrast correction | Actual confirmation hint color `rgb(72,85,99)` on panel `rgb(227,231,234)` = 6.13:1; final screenshot captured |
| Data retention | Existing emulator balance remained 920 THB after APK updates. Verification draft cleared and overview left open |

The palette's tested text pairs range from 5.57:1 to 15.11:1; full pair list is in the design document. This is a focused contrast check, not a full WCAG audit. Transaction signs and labels remain alongside semantic colors.

## Screenshots

- `.preview/dark-doodle-desktop.png`: final doodle overview at desktop size.
- `.preview/dark-doodle-android.png`: final installed Android overview.
- `.preview/dark-doodle-confirmation.png`: corrected light receipt panel with dark explanatory text.

Temporary browser/CDP sessions and the Android debug port forward were closed; the reference-site tab was closed. The finished desktop test app and Android test app were left available. No personal Windows database or Android ledger records were deleted.

This remains a debug emulator build. This UI work does not implement Android OCR, local LLM, tax calculations, release signing or verify ARM64/physical devices. Existing feature limits in the Android guide still apply.
