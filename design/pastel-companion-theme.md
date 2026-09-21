# Pastel companion theme — 21 September 2026

Based on the latest user-supplied ZenZ UI reference: lavender, pink, and peach gradients; white rounded cards; gentle shadows; pastel category tiles; a light bottom navigation bar. This replaces the warm yellow theme.

| Role | Color |
| --- | --- |
| Canvas / Android status bar | `#F7F1FC` |
| Cards / navigation bar | `#FFFDFD` |
| Soft surfaces | `#F2EBFC` |
| Primary text | `#30243E` |
| Secondary text | `#6B5C78` |
| Action gradient | `#BDA0FF` → `#F2AFD1` → `#FFC3A8` |
| Hero gradient | `#E8DCFF` → `#FBE6F2` → `#FFF0E5` |
| Positive / negative amounts | `#246859` / `#A33961` |

Use dark text over pastel action gradients to keep labels legible. Card text stays on a solid near-white surface. Decorative gradients and illustration shadows provide depth; there is no continuous background animation. Charts keep category labels and amounts alongside their colors.

Retain TH Sarabun New, the original monochrome tanuki doodles, manual entry, and the centered full-width mobile account cards. Category SVG colors use shared CSS variables with pink, lavender, sky, mint, and peach variations. Mobile overview artwork is smaller to give the balance and monthly information more room.

This adapts the reference's visual language to the ledger's existing features; it does not add the study app's copy, logo, or features.

## Verification

`cargo fmt --all -- --check`, UI Clippy with `-D warnings`, and the Android debug build passed. Checked overview, accounts, and manual entry in the Pixel_6 Android emulator at 411 CSS pixels: no horizontal overflow; account cards still fill their grid and center their content; manual date remains 2026-09-21. The existing cash balance stayed at THB 1,000 and no transactions were written during theme checks.

Calculated contrast for the selected solid palette pairs: primary text on the action gradient's stops is at least 6.66:1; secondary text on card/soft/hero-stop surfaces is at least 4.70:1. This is a palette check, not a full accessibility audit.

Actual emulator screenshots are saved in `.preview/pastel-overview.png`, `.preview/pastel-accounts.png`, and `.preview/pastel-manual.png` at the project root. The x86_64 debug APK is `target/android/ledgers-bro-test-x86_64.apk` with SHA-256 `eb08a8f06dbc1b343b8692fe0c263a2b78ba11cbccb26d9bc3d1395403e4af89`.
