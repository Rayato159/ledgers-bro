#!/usr/bin/env python3
"""Build the Windows x64 MSI, including local receipt OCR (Python 3.10+)."""
import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
LANGUAGES = {
    'tha': '294227cc2d1292b0acb28d61d4115c88252b96d466ca90b417cf4cf0c67bf07c',
    'eng': '7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2',
}


def digest(path):
    checksum = hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            checksum.update(block)
    return checksum.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ocr-dir', type=Path, default=ROOT / '.tools/ocr')
    parser.add_argument('--check', action='store_true', help='check prerequisites without building')
    args = parser.parse_args()
    if os.name != 'nt':
        raise RuntimeError('Run this MSI build on Windows with the MSVC Rust toolchain.')
    for command in ['cargo', 'dx', 'cmake']:
        if not shutil.which(command):
            raise RuntimeError(f'Missing {command}; see docs/windows-install-th.md')
    if subprocess.check_output(['dx', '--version'], text=True).split()[:2] != ['dioxus', '0.7.2']:
        raise RuntimeError('This packaging script is verified with Dioxus CLI 0.7.2.')

    environment = os.environ.copy()
    if not environment.get('LIBCLANG_PATH'):
        llvm = Path(environment.get('ProgramFiles', 'C:/Program Files')) / 'LLVM/bin'
        sdk = Path(environment.get('ANDROID_HOME') or environment.get('ANDROID_SDK_ROOT')
                   or Path(environment['LOCALAPPDATA']) / 'Android/Sdk')
        candidates = [llvm, *sorted(sdk.glob('ndk/*/toolchains/llvm/prebuilt/windows-x86_64/bin'), reverse=True)]
        found = next((path for path in candidates if (path / 'libclang.dll').is_file()), None)
        if found:
            environment['LIBCLANG_PATH'] = str(found)
    if not (Path(environment.get('LIBCLANG_PATH', '')) / 'libclang.dll').is_file():
        raise RuntimeError('Set LIBCLANG_PATH to the directory containing libclang.dll.')

    ocr = args.ocr_dir.resolve()
    required = [ocr / 'tesseract.exe', ocr / 'doc/LICENSE',
                ocr / 'image-decoder/magick.exe', ocr / 'image-decoder/LICENSE.txt',
                ocr / 'image-decoder/NOTICE.txt', ROOT / 'design/app-icon/launcher.ico']
    for path in required:
        if not path.is_file():
            raise RuntimeError(f'Missing runtime file: {path}')
    if not list(ocr.glob('*.dll')):
        raise RuntimeError('The Tesseract runtime DLLs are missing.')
    for language, expected in LANGUAGES.items():
        path = ocr / f'tessdata/{language}.traineddata'
        if not path.is_file() or digest(path) != expected:
            raise RuntimeError(f'Missing or unexpected OCR language data: {path}')
    subprocess.run([str(ocr / 'tesseract.exe'), '--version'], check=True, env=environment)
    subprocess.run([str(ocr / 'image-decoder/magick.exe'), '-version'], check=True, env=environment)
    if args.check:
        print('Windows installer prerequisites are ready.', flush=True)
        return

    # Only remove our generated staging directory, never a caller-supplied path.
    target = (ROOT / 'target').resolve()
    stage = target / 'windows-resources'
    if target.parent != ROOT or stage.is_symlink() or stage.resolve().parent != target:
        raise RuntimeError('The resource staging directory must stay inside target/.')
    if ocr.is_relative_to(stage):
        raise RuntimeError('The OCR source must be outside the generated staging directory.')
    if stage.exists():
        shutil.rmtree(stage)
    runtime = stage / 'ocr'
    runtime.mkdir(parents=True)
    for path in [ocr / 'tesseract.exe', *ocr.glob('*.dll')]:
        shutil.copy2(path, runtime / path.name)
    (runtime / 'tessdata').mkdir()
    for language in LANGUAGES:
        shutil.copy2(ocr / f'tessdata/{language}.traineddata', runtime / 'tessdata')
    # Preserve the complete unmodified portable decoder and its third-party notices.
    shutil.copytree(ocr / 'image-decoder', runtime / 'image-decoder')
    shutil.copytree(ocr / 'doc', runtime / 'doc')
    shutil.copytree(ROOT / 'licenses', stage / 'licenses')
    shutil.copy2(ROOT / 'LICENSE', stage / 'licenses/Ledgers-Bro-LICENSE.txt')
    shutil.copy2(ROOT / 'crates/ui/assets/fonts/LICENSE.txt', stage / 'licenses/Font-LICENSE.txt')

    output = target / 'installers'
    output.mkdir(parents=True, exist_ok=True)
    print('Building the release MSI; the first build can take several minutes.', flush=True)
    subprocess.run(['dx', 'bundle', '--desktop', '--release', '--package', 'ledgers-bro',
                    '--package-types', 'msi', '--locked', '--out-dir', str(output)],
                   cwd=ROOT / 'apps/desktop', env=environment, check=True)
    print(f'Installer directory: {output}', flush=True)


if __name__ == '__main__':
    try:
        main()
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f'Windows installer build failed: {error}', file=sys.stderr)
        sys.exit(1)
