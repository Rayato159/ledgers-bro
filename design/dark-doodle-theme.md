# Dark slate + doodle tanuki — 21 September 2026

Historical palette; see [Pastel companion](pastel-companion-theme.md) for the current colors. The tanuki artwork and its generation prompts below remain in use.

## Accepted direction

The user's bill-splitter reference supplies rounded cards, inset controls, a floating mobile navigation bar and a receipt-like confirmation panel. The live dark version of [Dancing With My Code](https://dancingwithmycode.com/) supplies the slate background and surface family, inspected in the browser on this date. The user's later instructions replace the website's gold accent with white/gray and replace the initial 3D mascot direction with hand-drawn doodles.

- Background: `#151C25`; card: `#252E39`; inset surface: `#1B242F`.
- Main text: `#F0F1ED`; secondary text: `#B4BDC7`.
- Primary action: `#E3E7EA` with `#1B242E` text.
- Muted green and red carry transaction/status meaning alongside labels and +/− signs; they are not primary brand accents.
- Tanuki art is grayscale pen doodles, flat off-white fills, gray patches and imperfect black outlines. No gold, purple or 3D fur.
- TH Sarabun New remains bundled. Body and small text tokens remain 24px and 20px before user zoom; a few compact mobile labels use 18.4px.
- Old room/human mascot copies were removed during repository cleanup after confirming neither host uses them. The user's original drawings outside this project were not changed.

These choices use low-chroma related slate surfaces for hierarchy and large luminance differences for actions and text. Readability was checked numerically rather than assumed from a color-wheel scheme. This is not a full accessibility certification.

## Final assets

Generated with the built-in image generation tool, then copied into the shared UI package. No external image service is used by the running app.

- `crates/ui/assets/tanuki/tanuki-ledger.png`: overview, accounts and history.
- `crates/ui/assets/tanuki/tanuki-phone.png`: quick-entry greeting.
- `ArtAssets::bundled()` supplies the same two images to Windows and Android, replacing host-specific human mascot/room includes.
- Existing editable SVG category drawings were recolored to slate/gray and given heavier outlines; they are code assets, not generated raster stand-ins.

### Final ledger prompt

Use case: stylized-concept. Asset type: transparent 2D hand-drawn DOODLE mascot for a charcoal dark finance app. Draw ONE adorable little Japanese tanuki raccoon-dog sitting and hugging a small accounting notebook. Style is explicitly a simple PEN DOODLE: loose confident wobbly thick black ink contours, primitive rounded shapes, playful imperfect hand-sketched linework, flat off-white paper-like solid fills and a few flat light-gray patches. Tiny simple dot eyes with black mask patches, tiny smile, two small rounded ears, little leaf on top, round belly, short rounded paws, two visible short feet, ONE plain fluffy rounded tail without stripes. The notebook has a tiny simple outlined square on the cover. Keep anatomy simple and coherent. Use pure grayscale ONLY: almost-black lines, off-white fills, mid gray face patches. Absolutely no brown, yellow, gold, purple or green. NO 3D rendering, NO realistic fur, NO gradients, NO lighting effects, NO shading volumes, NO glossy plush toy look, NO realistic eyes, NO fine fur texture. Charm comes from expressive hand-drawn doodle lines and minimalism. Strong legibility at 100px. Full body centered within square, generous 10% transparent margin, all extremities visible. Truly transparent alpha background around the character, no paper rectangle, no backdrop, no drop shadow, no words/letters/numbers/watermark, no UI or collage. Deliver single clean PNG cutout.

### Final phone prompt

Reference: the generated ledger doodle, not the discarded 3D concept.

Edit this image into a second pose of the SAME tanuki mascot. Keep this exact simple hand-drawn black-ink DOODLE style, imperfect wobbly thick outlines, off-white flat filled body, gray face patches, dot eyes, tiny smile, leaf on head, rounded ears and one unstriped fluffy tail. Change pose to standing upright on two short feet, holding a small off-white and gray smartphone in one paw and waving hello with the other paw. Simple coherent animal anatomy: exactly two arms and two legs, no extra paws. The phone screen is plain light gray, no text or symbols. Match this image's primitive shapes and line weight, not a 3D model. Grayscale ONLY, no purple, gold, brown, green or colored accents. Absolutely NO realistic fur, gradients, volume shading, glossy surfaces, photorealistic eyes, or 3D rendering. Full single character centered in a square with 10% transparent margin, all ears, leaf, tail and feet in frame. Transparent alpha background preserved; no white background rectangle, floor, shadow, scenery, words, numbers, labels, watermark or extra animals. Clean PNG cutout for a dark expense-tracking UI.

## Text contrast checks

Ratios calculated from sRGB relative luminance for these explicit foreground/background token pairs. All listed pairs exceed 4.5:1.

| Use | Foreground | Background | Ratio |
|---|---|---|---|
| body | `#f0f1ed` | `#151c25` | 15.11:1 |
| card text | `#f0f1ed` | `#252e39` | 12.11:1 |
| muted card | `#b4bdc7` | `#252e39` | 7.23:1 |
| muted raised | `#b4bdc7` | `#303c49` | 5.91:1 |
| primary button | `#1b242e` | `#e3e7ea` | 12.62:1 |
| receipt labels | `#485563` | `#e3e7ea` | 6.13:1 |
| income | `#a9d0bc` | `#252e39` | 8.14:1 |
| expense | `#edb7b5` | `#252e39` | 7.88:1 |
| error | `#f2c5c3` | `#443238` | 7.7:1 |
| selected category | `#f0f1ed` | `#394756` | 8.38:1 |
| destructive button | `#ffffff` | `#a84b49` | 5.57:1 |

## Scope

Presentation CSS, component composition/copy, bundled assets and native window/system-bar colors only. No accounting formulas, journal invariants, database schema, OCR engine or tax behavior changed. Android OCR/LLM/tax limitations remain as described in the Android guide.
