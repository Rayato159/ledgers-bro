#!/bin/sh
# Development runtime for local JPG/PNG OCR on macOS; not a redistributable bundle.
set -eu
runtime_root="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)/.tools/ocr"
tesseract_path="$(command -v tesseract || true)"
if [ -z "$tesseract_path" ]; then
    echo 'Install Tesseract first: brew install tesseract' >&2
    exit 1
fi
mkdir -p "$runtime_root/tessdata"
if [ ! -e "$runtime_root/tesseract" ]; then
    ln -s "$tesseract_path" "$runtime_root/tesseract"
fi
model_revision=87416418657359cb625c412a48b6e1d6d41c29bd
fetch_model() {
    model_name="$1"
    expected_hash="$2"
    model_path="$runtime_root/tessdata/$model_name.traineddata"
    if [ -f "$model_path" ] && [ "$(shasum -a 256 "$model_path" | cut -d ' ' -f 1)" = "$expected_hash" ]; then
        return
    fi
    model_temp="$(mktemp "$runtime_root/tessdata/download.XXXXXX")"
    trap 'rm -f "$model_temp"' EXIT HUP INT TERM
    curl --fail --location --proto '=https' "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/$model_revision/$model_name.traineddata" -o "$model_temp"
    if [ "$(shasum -a 256 "$model_temp" | cut -d ' ' -f 1)" != "$expected_hash" ]; then
        echo 'OCR model checksum mismatch' >&2
        exit 1
    fi
    mv "$model_temp" "$model_path"
    trap - EXIT HUP INT TERM
}
fetch_model tha 294227cc2d1292b0acb28d61d4115c88252b96d466ca90b417cf4cf0c67bf07c
fetch_model eng 7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2
"$runtime_root/tesseract" --list-langs --tessdata-dir "$runtime_root/tessdata"
