# Current design assets

The active mascot is Lookhin's original **uncle crab**. The earlier tanuki and modern-fantasy concepts are retired. Their unused studies and master images have been removed from the working tree; Git history preserves earlier revisions.

| Location | Purpose |
| --- | --- |
| [uncle-crab](uncle-crab/README.md) | Character identity, generation prompts and six original generated pose masters |
| [app-icon](app-icon/README.md) | Windows launcher exports derived from the crab hero pose |
| `../crates/ui/assets/uncle-crab/` | Sized runtime illustrations |
| `../crates/ui/assets/app.css` | Current layout, typography, responsive rules and theme tokens |
| `../crates/ui/src/artwork.rs` | Runtime artwork mapping and category icon components |

Run `python scripts/prepare-art-assets.py` from the repository root to reproduce runtime exports and launcher resources. This requires Pillow and preserves the master images. Keep source icon files: packaging references them even after build output is cleaned.

## UI direction

Use readable text, consistent row/card spacing and clear money alignment in both light and dark themes. Organize long screens into focused tabbed groups, following Settings. Use review modals for editing and confirmation, and show complete details on selection rather than expanding every list row. Charts must expose names, amounts and percentages through hover/focus and remain usable on touch screens.

Keep the mascot in existing branding and illustration areas. Do not restore removed promotional banners or place decorative characters beside action buttons. The quick-entry composer keeps voice, examples and model controls compact; model removal belongs in Settings.

Do not place screenshots containing personal or test financial data in this public design directory. Reusable artwork masters are separate from private QA captures.
