#!/usr/bin/env python3
"""
Browser Picker Daemon — keeps tkinter warm and shows the picker on demand.

Run at Windows startup (install.py sets this up).
Listens on TCP 127.0.0.1:27384 for URLs from the launcher.
"""

import os
import sys
import socket
import threading
import tkinter as tk

_here = os.path.dirname(sys.executable if getattr(sys, 'frozen', False) else os.path.abspath(__file__))
sys.path.insert(0, _here)

from browser_picker import (
    BG, BG_CARD, BG_HOVER, BORDER, FG, FG_DIM,
    MAX_VISIBLE_CARDS, CARD_HEIGHT,
    cursor_monitor_workarea,
    discover_profiles, launch, auto_launch_for_url,
    make_card, open_rules_manager, _set_bg, _all_widgets,
)
import browser_picker_rules as rules

PORT = 27384


# ── Picker window (Toplevel variant for daemon) ────────────────────────────────

class PickerWindow:
    def __init__(self, master, url, profiles=None):
        self.url = url
        self.profiles = profiles if profiles is not None else discover_profiles()
        self._done = False
        self._images = []   # keep PhotoImage refs alive

        win = tk.Toplevel(master)
        win.title('Browser Picker')
        win.configure(bg=BG)
        win.resizable(False, False)
        win.attributes('-topmost', True)
        win.withdraw()
        self.win = win

        self._build_ui()
        self._center()
        self._bind_keys()
        win.deiconify()
        win.lift()
        win.focus_force()

    def _build_ui(self):
        win = self.win

        hdr = tk.Frame(win, bg=BG, padx=16, pady=14)
        hdr.pack(fill='x')
        tk.Label(hdr, text='Open link in…',
                 font=('Segoe UI', 12, 'bold'), bg=BG, fg=FG).pack(anchor='w')
        url_text = self.url if len(self.url) <= 60 else self.url[:57] + '…'
        tk.Label(hdr, text=url_text, font=('Segoe UI', 9),
                 bg=BG, fg=FG_DIM).pack(anchor='w', pady=(3, 0))

        tk.Frame(win, bg=BORDER, height=1).pack(fill='x')

        n = len(self.profiles)
        viewport_h = min(n, MAX_VISIBLE_CARDS) * CARD_HEIGHT + 16

        canvas = tk.Canvas(win, bg=BG, highlightthickness=0,
                           height=viewport_h, width=440)
        canvas.pack(fill='both', side='left', expand=True)

        if n > MAX_VISIBLE_CARDS:
            sb = tk.Scrollbar(win, orient='vertical', command=canvas.yview,
                              bg=BG, troughcolor=BG_CARD, activebackground=BG_HOVER)
            sb.pack(side='right', fill='y')
            canvas.configure(yscrollcommand=sb.set)

        inner = tk.Frame(canvas, bg=BG, padx=10, pady=8)
        wid = canvas.create_window(0, 0, anchor='nw', window=inner)

        inner.bind('<Configure>', lambda e: (
            canvas.configure(scrollregion=canvas.bbox('all')),
            canvas.itemconfigure(wid, width=canvas.winfo_width()),
        ))
        canvas.bind('<Configure>', lambda e: canvas.itemconfigure(wid, width=e.width))
        canvas.bind_all('<MouseWheel>',
                        lambda e: canvas.yview_scroll(int(-1 * e.delta / 120), 'units'))

        domain_rule = rules.find_domain_rule(rules.domain_of(self.url))
        for idx, profile in enumerate(self.profiles):
            make_card(inner, idx, profile, self._images, on_select=self._select,
                      url=self.url, domain_rule=domain_rule).pack(fill='x', pady=2)

        tk.Frame(win, bg=BORDER, height=1).pack(fill='x')
        ftr = tk.Frame(win, bg=BG, padx=16, pady=8)
        ftr.pack(fill='x')
        hint = ('1–9 to pick  ·  scroll for more  ·  Esc to cancel'
                if n > 9 else '1–9 to pick  ·  Esc to cancel')
        tk.Label(ftr, text=hint, font=('Segoe UI', 8), bg=BG, fg=FG_DIM).pack(anchor='w', side='left')
        manage = tk.Label(ftr, text='Manage defaults…', font=('Segoe UI', 8),
                          bg=BG, fg=FG_DIM, cursor='hand2')
        manage.pack(anchor='e', side='right')
        manage.bind('<Button-1>', lambda _: open_rules_manager(win, self.url))

    def _select(self, profile):
        if not self._done:
            self._done = True
            launch(profile, self.url)
            self.win.destroy()

    def _bind_keys(self):
        self.win.bind('<Escape>', lambda _: self.win.destroy())
        for idx, profile in enumerate(self.profiles):
            key = str(idx + 1) if idx < 9 else ('0' if idx == 9 else None)
            if key:
                def _sel(_, p=profile):
                    self._select(p)
                self.win.bind(key, _sel)

    def _center(self):
        self.win.update_idletasks()
        w = max(460, self.win.winfo_reqwidth())
        h = self.win.winfo_reqheight()
        try:
            ml, mt, mr, mb = cursor_monitor_workarea()
        except Exception:
            ml, mt, mr, mb = 0, 0, self.win.winfo_screenwidth(), self.win.winfo_screenheight()
        x = ml + (mr - ml - w) // 2
        y = mt + (mb - mt - h) // 2
        self.win.geometry(f'{w}x{h}+{x}+{y}')


# ── Daemon ─────────────────────────────────────────────────────────────────────

_current_picker = None


def _show_picker(root, url):
    global _current_picker
    profiles = discover_profiles()
    if auto_launch_for_url(url, profiles):
        return
    if _current_picker is not None:
        try:
            _current_picker.win.destroy()
        except Exception:
            pass
    _current_picker = PickerWindow(root, url, profiles=profiles)


def _socket_server(root):
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    try:
        server.bind(('127.0.0.1', PORT))
    except OSError:
        return  # Another daemon instance is already running
    server.listen(5)
    while True:
        try:
            conn, _ = server.accept()
            data = b''
            conn.settimeout(2)
            try:
                while True:
                    chunk = conn.recv(4096)
                    if not chunk:
                        break
                    data += chunk
            except socket.timeout:
                pass
            conn.close()
            url = data.decode('utf-8', errors='replace').strip()
            if url:
                root.after(0, lambda u=url: _show_picker(root, u))
        except Exception:
            pass


def main():
    root = tk.Tk()
    root.withdraw()

    threading.Thread(target=discover_profiles, daemon=True).start()
    threading.Thread(target=_socket_server, args=(root,), daemon=True).start()
    root.mainloop()


if __name__ == '__main__':
    main()
