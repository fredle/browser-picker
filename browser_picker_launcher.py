#!/usr/bin/env python3
"""
Browser Picker Launcher — registered as the Windows default browser.

Sends the URL to the running daemon via TCP so the picker appears instantly.
Falls back to starting the daemon (which then shows the picker) if it's not running.
No tkinter import — keeps this exe small and fast to start.
"""

import os
import sys
import socket
import subprocess

PORT = 27384
TIMEOUT = 0.4   # seconds to wait for daemon connection


def _here():
    # Works both as a .py and as a PyInstaller frozen exe
    return os.path.dirname(sys.executable if getattr(sys, 'frozen', False) else os.path.abspath(__file__))


def send_to_daemon(url):
    """Return True if the URL was sent to the daemon successfully."""
    try:
        with socket.create_connection(('127.0.0.1', PORT), timeout=TIMEOUT) as s:
            s.sendall(url.encode('utf-8'))
        return True
    except (ConnectionRefusedError, socket.timeout, OSError):
        return False


def start_daemon_with_url(url):
    """Start the daemon process; it will show the picker for this URL on startup."""
    here = _here()
    daemon = os.path.join(here, 'browser_picker_daemon.exe')
    if not os.path.exists(daemon):
        # Dev fallback: run as Python script
        daemon_py = os.path.join(here, 'browser_picker_daemon.py')
        if os.path.exists(daemon_py):
            subprocess.Popen(
                [sys.executable, daemon_py],
                creationflags=subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP,
            )
            return
    else:
        subprocess.Popen(
            [daemon],
            creationflags=subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP,
        )

    # Give the daemon a moment to bind its socket, then send the URL
    import time
    for _ in range(10):
        time.sleep(0.2)
        if send_to_daemon(url):
            return

    # Last resort: open the full picker directly
    picker = os.path.join(here, 'browser_picker.exe')
    if os.path.exists(picker):
        subprocess.Popen([picker, url])


def main():
    url = sys.argv[1] if len(sys.argv) > 1 else 'https://example.com'

    if send_to_daemon(url):
        return  # Fast path — daemon was already running

    start_daemon_with_url(url)


if __name__ == '__main__':
    main()
