#!/usr/bin/env python3
"""Mechanical size/format exports only. Never redraw or recolor generated masters."""
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
MASTERS = ROOT / 'design/modern-fantasy/masters'
OUTPUT = ROOT / 'crates/ui/assets/characters'
OUTPUT.mkdir(parents=True, exist_ok=True)
for name in ('lumi', 'mint', 'peach', 'skye', 'iris', 'ren'):
    source = Image.open(MASTERS / f'{name}.png')
    if source.mode != 'RGBA' or source.getchannel('A').getextrema() != (0, 255):
        raise RuntimeError(f'{name}: expected generated transparency')
    source.thumbnail((768, 768), Image.Resampling.LANCZOS)
    source.save(OUTPUT / f'{name}.png', optimize=True)


def padded_portrait(size, fraction):
    portrait = Image.open(MASTERS / 'lumi.png').convert('RGBA')
    portrait.thumbnail((round(size * fraction), round(size * fraction)), Image.Resampling.LANCZOS)
    canvas = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    canvas.alpha_composite(portrait, ((size - portrait.width) // 2, (size - portrait.height) // 2))
    return canvas


icon_dir = ROOT / 'design/app-icon'
master = padded_portrait(1024, .88)
master.save(icon_dir / 'launcher-1024.png', optimize=True)
master.resize((512, 512), Image.Resampling.LANCZOS).save(icon_dir / 'launcher-512.png', optimize=True)
master.save(icon_dir / 'launcher.ico', sizes=[(n, n) for n in (16, 24, 32, 48, 64, 128, 256)])
android = ROOT / 'apps/android/native/res'
for density, size in [('mdpi', 48), ('hdpi', 72), ('xhdpi', 96), ('xxhdpi', 144), ('xxxhdpi', 192)]:
    master.resize((size, size), Image.Resampling.LANCZOS).save(android / f'mipmap-{density}/ledger_launcher.png', optimize=True)
padded_portrait(432, .60).save(android / 'drawable-nodpi/ledger_launcher_foreground.png', optimize=True)
print('Exported six RGBA characters and Windows/Android launcher sizes.')
