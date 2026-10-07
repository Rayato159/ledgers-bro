# Tanuki launcher icon

The tanuki salaryman is the application mascot. Its compact mark was generated with the built-in image generation tool from Lookhin's direction and original style references; see [artwork and prompts](../tanuki/README.md).

`launcher-1024.png`, `launcher-512.png` and `launcher.ico` are mechanical exports of the unmodified generated image in `../tanuki/masters/logo.png`. Android resources include an adaptive foreground with transparent safe margins and five legacy densities. Reproduce them with `python scripts/prepare-art-assets.py` (Pillow required).

The Windows installer and Android manifest reference these source-controlled resources so they survive cleaning build output. No separate monochrome Android layer is supplied.
