#!/usr/bin/env python3
"""Mechanical size/format exports only. Never redraw or recolor generated masters."""
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
MASTERS = ROOT / 'design/uncle-crab/masters'
OUTPUT = ROOT / 'crates/ui/assets/uncle-crab'
OUTPUT.mkdir(parents=True, exist_ok=True)
for name in ('hero', 'history', 'calendar', 'phone', 'tax', 'off-duty'):
    source = Image.open(MASTERS / f'{name}.png')
    alpha = source.getchannel('A').getextrema() if source.mode == 'RGBA' else None
    if alpha is None or alpha[0] != 0 or alpha[1] < 250:
        raise RuntimeError(f'{name}: expected generated transparency')
    source.thumbnail((640, 640), Image.Resampling.LANCZOS)
    source.save(OUTPUT / f'{name}.png', optimize=True)


def padded_portrait(size, fraction):
    portrait = Image.open(MASTERS / 'hero.png').convert('RGBA')
    portrait = portrait.crop(portrait.getchannel('A').getbbox())
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
print('Exported six RGBA uncle-crab poses and Windows/Android launcher sizes.')
