#!/usr/bin/env python3
"""Remove Browser Picker registry entries."""

import os
import sys
import winreg
from pathlib import Path

APP_NAME = 'BrowserPicker'
PROG_ID  = 'BrowserPickerURL'
SHORTCUT_NAME = 'Browser Picker.lnk'


def _delete_tree(hive, path):
    """Delete a registry key and all its subkeys (like reg delete /f)."""
    try:
        with winreg.OpenKey(hive, path, access=winreg.KEY_READ | winreg.KEY_WRITE) as k:
            while True:
                try:
                    child = winreg.EnumKey(k, 0)
                    _delete_tree(hive, rf'{path}\{child}')
                except OSError:
                    break
        winreg.DeleteKey(hive, path)
    except FileNotFoundError:
        pass  # already gone


def _delete_value(hive, path, name):
    try:
        with winreg.OpenKey(hive, path, access=winreg.KEY_SET_VALUE) as k:
            winreg.DeleteValue(k, name)
    except FileNotFoundError:
        pass


def remove_start_menu_shortcut():
    lnk = (Path(os.environ['APPDATA']) / 'Microsoft' / 'Windows'
           / 'Start Menu' / 'Programs' / SHORTCUT_NAME)
    lnk.unlink(missing_ok=True)


def unregister():
    hkcu = winreg.HKEY_CURRENT_USER

    _delete_tree(hkcu,  rf'Software\Classes\{PROG_ID}')
    _delete_tree(hkcu,  rf'Software\Clients\StartMenuInternet\{APP_NAME}')
    _delete_value(hkcu, r'Software\RegisteredApplications', APP_NAME)

    remove_start_menu_shortcut()

    print('Browser Picker unregistered.')
    print()
    print('Remember to set a different default browser in:')
    print('  Settings > Apps > Default apps')


if __name__ == '__main__':
    try:
        unregister()
    except Exception as exc:
        print(f'Error: {exc}')
        sys.exit(1)
