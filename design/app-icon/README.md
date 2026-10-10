# Studio launcher icon

The compact face mark emphasizes the mascot's glasses and hair so it remains recognizable at small sizes. See the [artwork provenance and prompts](../studio/README.md).

`launcher-1024.png`, `launcher-512.png` and the multi-size `launcher.ico` are mechanical exports of `../studio/masters/logo.png`. Android includes an adaptive foreground with transparent safe margins, a warm paper background, and five legacy densities. Run `python scripts/prepare-art-assets.py` with Pillow to reproduce them.

Windows embeds the ICO in the executable and loads that resource for its window. The MSI also references it for installer branding. Android's manifest references the launcher resources. These source assets survive build cleanup. No separate monochrome Android layer is supplied.
