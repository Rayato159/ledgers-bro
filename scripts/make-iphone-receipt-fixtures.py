"""Synthetic format fixtures, not photographs captured on an iPhone.
Run with Pillow 12.1.0 and pillow-heif 1.3.0; no runtime Python dependency.
"""
from pathlib import Path
from PIL import Image
from pillow_heif import register_heif_opener, from_bytes

register_heif_opener()
root = Path(__file__).resolve().parent.parent / "tests" / "fixtures"
with Image.open(root / "receipt-th-en.png") as original:
    upright = original.convert("RGB")
    upright.save(root / "receipt-th-en.heic", quality=95)
    # Stored landscape pixels + EXIF rotation should display as upright portrait.
    sideways = upright.transpose(Image.Transpose.ROTATE_90)
    exif = Image.Exif()
    exif[274] = 6
    exif[270] = "SYNTHETIC RECEIPT FIXTURE"
    sideways.save(root / "receipt-exif6.jpg", quality=95, exif=exif)
    # HEIF rotation must be a container irot transform, not only EXIF metadata.
    from_bytes("RGB", sideways.size, sideways.tobytes()).save(
        root / "receipt-rotated.heic", quality=95, exif=exif.tobytes()
    )
    # Default 24 MP camera size must be downsampled, not silently rejected.
    upright.resize((4000, 6000)).save(root / "receipt-24mp.heic", quality=75)
print("Created JPEG orientation and HEIC fixtures from synthetic receipt.")
