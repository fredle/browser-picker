#!/usr/bin/env python3
"""
Register Browser Picker as a Windows browser and add the daemon to startup.

Run once from the folder that contains browser_picker_launcher.exe and
browser_picker_daemon.exe (after running build.bat).
"""

import os
import sys
import winreg
import subprocess
from pathlib import Path

APP_NAME = 'BrowserPicker'
PROG_ID  = 'BrowserPickerURL'
STARTUP_NAME = 'BrowserPickerDaemon'
SHORTCUT_NAME = 'Browser Picker.lnk'


def _here():
    return Path(sys.argv[0]).resolve().parent


def get_launcher_command():
    here = _here()
    exe = here / 'browser_picker_launcher.exe'
    if exe.exists():
        return f'"{exe}" "%1"'
    # Dev fallback
    script = here / 'browser_picker_launcher.py'
    return f'"{sys.executable}" "{script}" "%1"'


def get_daemon_command():
    here = _here()
    exe = here / 'browser_picker_daemon.exe'
    if exe.exists():
        return f'"{exe}"'
    script = here / 'browser_picker_daemon.py'
    return f'"{sys.executable}" "{script}"'


def _set(key, name, value):
    winreg.SetValueEx(key, name, 0, winreg.REG_SZ, value)


def register_browser():
    cmd = get_launcher_command()
    caps_path = rf'Software\Clients\StartMenuInternet\{APP_NAME}\Capabilities'

    # ProgID
    with winreg.CreateKeyEx(winreg.HKEY_CURRENT_USER,
                            rf'Software\Classes\{PROG_ID}',
                            access=winreg.KEY_WRITE) as k:
        _set(k, '', 'Browser Picker URL')
        _set(k, 'URL Protocol', '')

    with winreg.CreateKeyEx(winreg.HKEY_CURRENT_USER,
                            rf'Software\Classes\{PROG_ID}\shell\open\command',
                            access=winreg.KEY_WRITE) as k:
        _set(k, '', cmd)

    # StartMenuInternet entry
    with winreg.CreateKeyEx(winreg.HKEY_CURRENT_USER,
                            rf'Software\Clients\StartMenuInternet\{APP_NAME}',
                            access=winreg.KEY_WRITE) as k:
        _set(k, '', 'Browser Picker')

    with winreg.CreateKeyEx(winreg.HKEY_CURRENT_USER,
                            rf'Software\Clients\StartMenuInternet\{APP_NAME}\shell\open\command',
                            access=winreg.KEY_WRITE) as k:
        _set(k, '', cmd)

    with winreg.CreateKeyEx(winreg.HKEY_CURRENT_USER, caps_path,
                            access=winreg.KEY_WRITE) as k:
        _set(k, 'ApplicationName', 'Browser Picker')
        _set(k, 'ApplicationDescription',
             'Choose which browser profile to open links in')

    with winreg.CreateKeyEx(winreg.HKEY_CURRENT_USER,
                            rf'{caps_path}\URLAssociations',
                            access=winreg.KEY_WRITE) as k:
        _set(k, 'http',  PROG_ID)
        _set(k, 'https', PROG_ID)

    with winreg.OpenKey(winreg.HKEY_CURRENT_USER, r'Software\RegisteredApplications',
                        access=winreg.KEY_SET_VALUE) as k:
        winreg.SetValueEx(k, APP_NAME, 0, winreg.REG_SZ, caps_path)


def start_menu_dir():
    return (Path(os.environ['APPDATA']) / 'Microsoft' / 'Windows'
            / 'Start Menu' / 'Programs')


def create_start_menu_shortcut():
    """Add a Start Menu entry that opens the default-browser rules manager."""
    here = _here()
    exe = here / 'browser_picker.exe'
    if exe.exists():
        target, args = str(exe), '--manage'
    else:
        # Dev fallback
        target, args = sys.executable, f'"{here / "browser_picker.py"}" --manage'

    lnk = start_menu_dir() / SHORTCUT_NAME
    lnk.parent.mkdir(parents=True, exist_ok=True)

    def q(value):
        return str(value).replace("'", "''")

    script = (
        f"$s = (New-Object -ComObject WScript.Shell).CreateShortcut('{q(lnk)}');"
        f"$s.TargetPath = '{q(target)}';"
        f"$s.Arguments = '{q(args)}';"
        f"$s.WorkingDirectory = '{q(here)}';"
        "$s.Description = 'Manage which browser profile opens which sites';"
        "$s.Save()"
    )
    subprocess.run(['powershell', '-NoProfile', '-NonInteractive', '-Command', script],
                   check=True, capture_output=True)
    return lnk


def register_startup():
    """Add the daemon to HKCU Run so it starts with Windows."""
    cmd = get_daemon_command()
    with winreg.OpenKey(winreg.HKEY_CURRENT_USER,
                        r'Software\Microsoft\Windows\CurrentVersion\Run',
                        access=winreg.KEY_SET_VALUE) as k:
        winreg.SetValueEx(k, STARTUP_NAME, 0, winreg.REG_SZ, cmd)


def start_daemon_now():
    """Launch the daemon immediately so the first click is fast."""
    here = _here()
    daemon = here / 'browser_picker_daemon.exe'
    if daemon.exists():
        subprocess.Popen(
            [str(daemon)],
            creationflags=subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP,
        )
        return True
    return False


if __name__ == '__main__':
    try:
        register_browser()
        print('OK  Browser Picker registered as a browser.')

        register_startup()
        print('OK  Daemon added to Windows startup.')

        print(f'OK  Start Menu shortcut created: {create_start_menu_shortcut()}')

        if start_daemon_now():
            print('OK  Daemon started -- first click will be fast.')

        print()
        print('Final step: set Browser Picker as your default browser.')
        print('Opening Windows Default Apps settings...')
        print('  Find "Browser Picker" and set it as default for HTTP and HTTPS.')
        subprocess.Popen(['cmd', '/c', 'start', 'ms-settings:defaultapps'], shell=False)

    except PermissionError as exc:
        print(f'Permission error: {exc}')
        print('Try running as Administrator.')
        sys.exit(1)
    except Exception as exc:
        print(f'Error: {exc}')
        sys.exit(1)
