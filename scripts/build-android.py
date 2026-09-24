#!/usr/bin/env python3
"""Build the test APK on macOS, Linux, or Windows (Python 3.10+)."""
import argparse
import hashlib
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
TOOLS = ROOT / '.tools'
TARGETS = {'aarch64-linux-android': ('arm64-v8a', 'arm64'), 'x86_64-linux-android': ('x86_64', 'x86_64')}
REVISION = '87416418657359cb625c412a48b6e1d6d41c29bd'
MODELS = {'tha': '294227cc2d1292b0acb28d61d4115c88252b96d466ca90b417cf4cf0c67bf07c',
          'eng': '7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2'}


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest() if sys.version_info >= (3, 11) else hashlib.sha256(source.read()).hexdigest()


def download(url, path, expected):
    if path.is_file() and digest(path) == expected:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + '.tmp')
    try:
        with urllib.request.urlopen(url, timeout=120) as source, temporary.open('wb') as output:
            shutil.copyfileobj(source, output)
        if digest(temporary) != expected:
            raise RuntimeError(f'Checksum mismatch: {path.name}')
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def latest(parent):
    candidates = [p for p in parent.glob('*') if p.is_dir() and re.fullmatch(r'\d+(\.\d+)*', p.name)]
    if not candidates:
        raise RuntimeError(f'Install the Android SDK component in {parent}')
    return max(candidates, key=lambda p: tuple(map(int, p.name.split('.'))))


def validate_apk(path, abi):
    with zipfile.ZipFile(path) as archive:
        libraries = [name for name in archive.namelist() if re.fullmatch(r'lib/[^/]+/[^/]+\.so', name)]
    if f'lib/{abi}/libdioxusmain.so' not in libraries:
        raise RuntimeError('APK is missing the application native library')
    if any(not name.startswith(f'lib/{abi}/') for name in libraries):
        raise RuntimeError('APK contains libraries for another ABI')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target', choices=TARGETS, default='aarch64-linux-android')
    parser.add_argument('--sdk', default=os.getenv('ANDROID_HOME') or os.getenv('ANDROID_SDK_ROOT'))
    parser.add_argument('--ndk', default=os.getenv('NDK_HOME'))
    parser.add_argument('--jdk', default=os.getenv('JAVA_HOME'))
    parser.add_argument('--check', action='store_true', help='check local tools without downloading or building')
    args = parser.parse_args()
    if not args.sdk or not args.jdk:
        raise RuntimeError('Set ANDROID_HOME and JAVA_HOME (JDK 21), or pass --sdk and --jdk')
    sdk, jdk = Path(args.sdk).resolve(), Path(args.jdk).resolve()
    ndk = Path(args.ndk).resolve() if args.ndk else latest(sdk / 'ndk')
    cmake = latest(sdk / 'cmake') / 'bin'
    host = {'Darwin': 'darwin-x86_64', 'Linux': 'linux-x86_64', 'Windows': 'windows-x86_64'}[platform.system()]
    llvm = ndk / 'toolchains/llvm/prebuilt' / host
    suffix = '.exe' if os.name == 'nt' else ''
    for required in [jdk / f'bin/java{suffix}', sdk / f'platform-tools/adb{suffix}', llvm / f'bin/clang{suffix}', cmake / f'ninja{suffix}']:
        if not required.is_file():
            raise RuntimeError(f'Missing tool: {required}')
    for command in ['dx', 'rustup']:
        if not shutil.which(command):
            raise RuntimeError(f'Install {command} first (Dioxus CLI must be 0.7.2)')
    version = subprocess.check_output(['dx', '--version'], text=True)
    if not re.search(r'\b0\.7\.2\b', version):
        raise RuntimeError('Install Dioxus CLI 0.7.2 to match this project')
    installed = subprocess.check_output(['rustup', 'target', 'list', '--installed'], text=True).splitlines()
    if args.target not in installed:
        raise RuntimeError(f'Run: rustup target add {args.target}')
    if args.check:
        print('Build tools found. No files downloaded or APK built.')
        return
    download('https://jitpack.io/cz/adaptech/tesseract4android/tesseract4android/4.9.0/tesseract4android-4.9.0.aar',
             TOOLS / 'downloads/tesseract4android-4.9.0.aar', 'bce5d6413a1a5ae3d7240033fbbc851ba3217d0a08d9769400e17a077f42cb2a')
    for language, expected in MODELS.items():
        destination = TOOLS / f'android-ocr/assets/tessdata/{language}.traineddata'
        cached = TOOLS / f'ocr/tessdata/{language}.traineddata'
        if cached.is_file() and digest(cached) == expected:
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(cached, destination)
        download(f'https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/{REVISION}/{language}.traineddata', destination, expected)
    gradle = TOOLS / 'android-gradle'
    (gradle / 'init.d').mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ROOT / 'apps/android/native/receipt.init.gradle', gradle / 'init.d/receipt.gradle')
    abi, label = TARGETS[args.target]
    env = os.environ.copy()
    env.update(JAVA_HOME=str(jdk), ANDROID_HOME=str(sdk), ANDROID_SDK_ROOT=str(sdk),
               NDK_HOME=str(ndk), ANDROID_NDK_HOME=str(ndk), ANDROID_NDK=str(ndk), ANDROID_API_LEVEL='24',
               LIBCLANG_PATH=str(llvm / ('bin' if os.name == 'nt' else 'lib')),
               CMAKE_GENERATOR='Ninja', GRADLE_USER_HOME=str(gradle),
               LEDGER_ANDROID_PROJECT_ROOT=str(ROOT), LEDGER_ANDROID_ABI=abi)
    for target in [args.target, args.target.replace('-', '_')]:
        env[f'BINDGEN_EXTRA_CLANG_ARGS_{target}'] = f'--target={args.target}24'
    env['PATH'] = os.pathsep.join([str(cmake), str(jdk / 'bin'), str(sdk / 'platform-tools'), env.get('PATH', '')])
    env['GRADLE_OPTS'] = env.get('GRADLE_OPTS', '') + ' -Dorg.gradle.daemon=false'
    if os.name == 'nt':
        temporary = TOOLS / 'java-tmp'
        temporary.mkdir(parents=True, exist_ok=True)
        env['JAVA_TOOL_OPTIONS'] = env.get('JAVA_TOOL_OPTIONS', '') + f' -Djava.io.tmpdir="{temporary}" -Djdk.net.unixdomain.tmpdir="{temporary}"'
    subprocess.run(['dx', 'build', '--platform', 'android', '--target', args.target, '--package', 'ledgers-bro-android',
                    '--no-default-features', '--features', 'mobile', '--locked'], cwd=ROOT / 'apps/android', env=env, check=True)
    apk = ROOT / 'target/dx/ledgers-bro-android/debug/android/app/app/build/outputs/apk/debug/app-debug.apk'
    validate_apk(apk, abi)
    output = ROOT / f'target/android/ledgers-bro-test-{label}.apk'
    output.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(apk, output)
    output.with_suffix('.sha256').write_text(f'{digest(output)}  {output.name}\n', encoding='utf-8')
    print(f'Test APK: {output}\nDebug signature; not a Store release.')


if __name__ == '__main__':
    try:
        main()
    except (OSError, RuntimeError, KeyError, subprocess.CalledProcessError) as error:
        sys.exit(str(error))
