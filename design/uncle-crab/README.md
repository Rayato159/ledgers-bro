# Uncle crab

Original character design and reference drawing by Lookhin. Colored variants were generated with the built-in image generation tool, using the user's drawing as the identity reference. This set supersedes the earlier modern-fantasy cast in the application.

The character is a mischievous, middle-aged salaryman: frugal, good at saving and kind underneath a skeptical expression. A wide orange shell, close-set claws, dot eyes, raised eyebrows, tiny moustache, cream chin and loosened lavender tie keep the poses consistent. His after-work pose includes a drink and cigarette, as requested by the character's creator.

| Pose | Use |
| --- | --- |
| `hero` | Login, overview, accounts and launcher icons; guarding savings |
| `history` | Manual entry and transaction history; ledger and pencil |
| `calendar` | Recurring bills; calendar and reminder pencil |
| `phone` | Quick entry and receivables; phone and a friendly gesture |
| `tax` | Tax; calculator and salary slip |
| `off-duty` | Settings; relaxing after work |

`masters/` preserves the selected generated PNGs unchanged. Runtime assets are in `../../crates/ui/assets/uncle-crab/`. Run `python scripts/prepare-art-assets.py` from the repository root for mechanical resizing, icon padding and format exports. The script preserves generated transparency and never redraws or recolors the artwork. The original reference drawing remains with its owner and is not copied into this repository.

See [the prompt set](prompts.md). Mascots occupy existing branding and illustration areas; previously removed promotional banners and decorations beside action buttons stay removed.
