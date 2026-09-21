# Receipt dependency notices

- Android: Tesseract4Android 4.9.0, Apache-2.0, https://github.com/adaptech-cz/Tesseract4Android/tree/4.9.0 . AAR checksum is pinned in `scripts/setup-android-ocr.ps1`.
- OCR language data: tessdata_fast revision `87416418657359cb625c412a48b6e1d6d41c29bd`, Apache-2.0, https://github.com/tesseract-ocr/tessdata_fast . Only Thai and English are bundled; their checksums are verified at build and installation.
- Windows HEIF decoder: unmodified ImageMagick 7.1.2-31 Q8 x64 portable runtime. `ImageMagick-LICENSE.txt` and the upstream aggregate `ImageMagick-NOTICE.txt` are retained. The runtime's complete directory and notices accompany desktop packaging. Source/release: https://github.com/ImageMagick/ImageMagick/releases/tag/7.1.2-31 . The Android app does not use this Windows executable.

Android AAR native components include Tesseract, Leptonica, libjpeg and libpng; the upstream build definitions describe exact versions. Commercial redistribution of complete native runtimes still requires checking their corresponding source/relinking obligations and third-party notices, especially libheif and its codec dependencies. These test builds are not a completed Store distribution audit.
