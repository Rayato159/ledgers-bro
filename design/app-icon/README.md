# Lumi launcher icon

The original modern-fantasy Lumi character replaces the tanuki in 0.1.2. Generated with Codex built-in image generation; see [cast and prompts](../modern-fantasy/README.md).

`launcher-1024.png`, `launcher-512.png` and `launcher.ico` are mechanical exports of the unmodified generated portrait in `../modern-fantasy/masters/lumi.png`. Android resources include adaptive foreground with transparent safe margins, a lavender background and five legacy densities. Reproduce them with `python scripts/prepare-art-assets.py` (Pillow required).

The Windows installer and Android manifest reference these source-controlled resources so they survive cleaning build output. No separate monochrome Android layer is supplied.
