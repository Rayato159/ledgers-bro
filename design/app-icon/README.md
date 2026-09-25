# Uncle crab launcher icon

Lookhin's original uncle crab is the application mascot. Its colored banking pose was generated with the built-in image generation tool; see [artwork and prompts](../uncle-crab/README.md).

`launcher-1024.png`, `launcher-512.png` and `launcher.ico` are mechanical exports of the unmodified generated image in `../uncle-crab/masters/hero.png`. Android resources include adaptive foreground with transparent safe margins, a lavender background and five legacy densities. Reproduce them with `python scripts/prepare-art-assets.py` (Pillow required).

The Windows installer and Android manifest reference these source-controlled resources so they survive cleaning build output. No separate monochrome Android layer is supplied.
