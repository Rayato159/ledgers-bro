# Windows packaging and installation

Run from the repository root on Windows x64. Install Python 3.10+, Rust MSVC, Visual Studio C++ Build Tools, CMake, libclang and Dioxus CLI 0.7.2.

```powershell
cargo install dioxus-cli --version 0.7.2 --locked
python scripts/build-windows.py --check
python scripts/build-windows.py
```

Use `py` if Python Launcher is configured. The helper locates libclang in LLVM or an Android SDK/NDK; set `LIBCLANG_PATH` to the folder containing `libclang.dll` for another installation.

Prepare the [OCR runtime](receipt-ocr.md) in `.tools/ocr`, or pass `--ocr-dir` with its directory. It must contain Tesseract and its DLLs, pinned Thai/English tessdata and the ImageMagick portable `image-decoder/` directory. The helper checks language checksums and invokes the runtimes before building.

The helper stages resources in `target/windows-resources/` and bundles an MSI into `target/installers/`. Initial bundling may download WiX and the WebView2 bootstrapper. Use the helper after cleaning output so OCR and license files are staged again.

Open the MSI and follow Windows installation prompts. Windows packages are unsigned. The app includes receipt OCR; optional [AI models](local-model-selection.md) are downloaded separately through the composer or Settings. Keep the product identity and `upgrade_code` in `apps/desktop/Dioxus.toml` stable across upgrades and increment the workspace version.

Default user data lives in `%LOCALAPPDATA%\Dancing With My Code\Ledgers Bro\data`, separate from the installed program. Back up before upgrading. Use `--data-dir .data/sandbox` and synthetic users for development; never launch a test build against the default real ledger.

The package contains the executable, UI assets, launcher icon, OCR executables/libraries, language files and licenses. It must not contain developer ledgers, exports, login tokens, screenshots from tests or downloaded LLM weights. A portable ZIP requires its `assets`, `ocr` and `licenses` folders beside the executable.
