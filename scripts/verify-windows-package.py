#!/usr/bin/env python3
"""Read the final MSI tables without installing it (Windows, Python 3.10+)."""
import argparse
import ctypes
from ctypes import wintypes
from pathlib import Path
import sys


def verify(path, version):
    msi = ctypes.WinDLL('msi')
    handle = wintypes.UINT()
    msi.MsiOpenDatabaseW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, ctypes.POINTER(wintypes.UINT)]
    msi.MsiDatabaseOpenViewW.argtypes = [wintypes.UINT, wintypes.LPCWSTR, ctypes.POINTER(wintypes.UINT)]
    msi.MsiRecordGetStringW.argtypes = [wintypes.UINT, wintypes.UINT, wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD)]

    def success(code):
        if code != 0:
            raise RuntimeError(f'Windows Installer database error {code}')

    def rows(table):
        view = wintypes.UINT()
        success(msi.MsiDatabaseOpenViewW(handle, f'SELECT * FROM `{table}`', ctypes.byref(view)))
        result = []
        try:
            success(msi.MsiViewExecute(view, 0))
            while True:
                record = wintypes.UINT()
                code = msi.MsiViewFetch(view, ctypes.byref(record))
                if code == 259:  # ERROR_NO_MORE_ITEMS
                    break
                success(code)
                try:
                    row = []
                    for field in range(1, msi.MsiRecordGetFieldCount(record) + 1):
                        buffer = ctypes.create_unicode_buffer(32768)
                        size = wintypes.DWORD(len(buffer))
                        success(msi.MsiRecordGetStringW(record, field, buffer, ctypes.byref(size)))
                        row.append(buffer.value)
                    result.append(row)
                finally:
                    msi.MsiCloseHandle(record)
        finally:
            msi.MsiCloseHandle(view)
        return result

    def require(condition, message):
        if not condition:
            raise RuntimeError(message)

    success(msi.MsiOpenDatabaseW(str(path.resolve()), None, ctypes.byref(handle)))
    try:
        properties = dict(rows('Property'))
        require(properties['ProductVersion'] == version, 'Wrong MSI version')
        require(properties['Manufacturer'] == 'dancingwithmycode.com', 'Wrong publisher')
        require(properties['UpgradeCode'].upper() == '{53588047-1F68-4BB5-B30A-7A5C18BCC514}', 'Changed upgrade identity')
        require(properties.get('ARPURLINFOABOUT') == 'https://dancingwithmycode.com', 'Missing publisher website')
        shortcuts = {row[0]: row for row in rows('Shortcut')}
        require(set(shortcuts) == {'ApplicationStartMenuShortcut', 'ApplicationDesktopShortcut'}, 'Unexpected installer shortcut')
        require(all(row[4] == '[!Path]' for row in shortcuts.values()), 'Invalid application shortcut target')
        require(any(row[0] == 'RemoveLegacyUninstallShortcut' and row[3] == 'INSTALLDIR' and row[4] == '1' for row in rows('RemoveFile')), 'Missing legacy shortcut cleanup')
        searches = rows('RegLocator')
        require(any(row[0] == 'PrevInstallDirWithName' and row[2] == r'Software\Dancing With My Code\LedgersBro' and row[3] == 'InstallDir' for row in searches), 'Changed legacy install-path lookup')
        require(any(row[2] == r'Software\Dancing With My Code\LedgersBro' and row[3] == 'InstallDir' and row[4] == '[INSTALLDIR]' for row in rows('Registry')), 'Changed install-path registry identity')
        sequence = {row[0]: int(row[2]) for row in rows('InstallExecuteSequence')}
        require(sequence['InstallInitialize'] < sequence['RemoveExistingProducts'] < sequence['ProcessComponents'], 'Unsafe upgrade removal order')
    finally:
        msi.MsiCloseHandle(handle)
    print('MSI publisher, upgrade identity, legacy path, shortcuts and cleanup verified.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('msi', type=Path)
    parser.add_argument('--version', required=True)
    args = parser.parse_args()
    try:
        verify(args.msi, args.version)
    except (OSError, RuntimeError, KeyError) as error:
        sys.exit(str(error))
