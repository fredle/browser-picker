#!/usr/bin/env python3
"""
Browser Picker — select a browser/profile when you click links.
Usage: browser_picker.py <URL>
       browser_picker.py --manage   # rules manager only
"""

import sys
import os
import json
import subprocess
import ctypes
import ctypes.wintypes
import tkinter as tk
from pathlib import Path

from PIL import Image, ImageDraw, ImageTk

import browser_picker_rules as rules

# ── Theme (Tokyo Night-inspired) ───────────────────────────────────────────────
BG       = '#1a1b26'
BG_CARD  = '#24283b'
BG_HOVER = '#2f3347'
FG       = '#c0caf5'
FG_DIM   = '#565f89'
BORDER   = '#3b4261'

AVATAR_COLORS = [
    '#f7768e', '#ff9e64', '#e0af68', '#9ece6a',
    '#73daca', '#7aa2f7', '#bb9af7', '#2ac3de',
    '#f7768e', '#ff9e64', '#e0af68', '#9ece6a',
    '#73daca', '#7aa2f7', '#bb9af7', '#2ac3de',
]
BROWSER_ACCENT = {'chrome': '#4285f4', 'edge': '#0f6cbd'}
BROWSER_LABEL  = {'chrome': 'Chrome',  'edge': 'Edge'}

AVATAR_SIZE       = 36   # px — large enough for photos to be recognisable
MAX_VISIBLE_CARDS = 9
CARD_HEIGHT       = 56

# ── Multi-monitor support ──────────────────────────────────────────────────────

class _RECT(ctypes.Structure):
    _fields_ = [('left', ctypes.wintypes.LONG), ('top', ctypes.wintypes.LONG),
                ('right', ctypes.wintypes.LONG), ('bottom', ctypes.wintypes.LONG)]

class _MONITORINFO(ctypes.Structure):
    _fields_ = [('cbSize', ctypes.wintypes.DWORD),
                ('rcMonitor', _RECT), ('rcWork', _RECT),
                ('dwFlags', ctypes.wintypes.DWORD)]

class _POINT(ctypes.Structure):
    _fields_ = [('x', ctypes.wintypes.LONG), ('y', ctypes.wintypes.LONG)]

def cursor_monitor_workarea():
    """Return (left, top, right, bottom) of the work area of whichever monitor the cursor is on."""
    user32 = ctypes.windll.user32
    pt = _POINT()
    user32.GetCursorPos(ctypes.byref(pt))
    hmon = user32.MonitorFromPoint(pt, 2)  # MONITOR_DEFAULTTONEAREST
    mi = _MONITORINFO()
    mi.cbSize = ctypes.sizeof(_MONITORINFO)
    user32.GetMonitorInfoW(hmon, ctypes.byref(mi))
    r = mi.rcWork
    return r.left, r.top, r.right, r.bottom


# ── Profile image helpers ──────────────────────────────────────────────────────

def _find_profile_image(profile_dir: Path) -> Path | None:
    """Return the best available profile photo in order of preference."""
    for name in ('Google Profile Picture.png', 'Edge Profile Picture.png',
                 'Google Profile.ico', 'Edge Profile.ico'):
        p = profile_dir / name
        if p.exists():
            return p
    return None


def make_circular_photo(path: Path, size: int = AVATAR_SIZE) -> ImageTk.PhotoImage:
    """Load an image file, resize to size×size, and crop it to a circle."""
    img = Image.open(path).convert('RGBA')
    img = img.resize((size, size), Image.LANCZOS)
    mask = Image.new('L', (size, size), 0)
    ImageDraw.Draw(mask).ellipse((0, 0, size - 1, size - 1), fill=255)
    out = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    out.paste(img, mask=mask)
    return ImageTk.PhotoImage(out)


def make_initial_photo(initial: str, color: str,
                       size: int = AVATAR_SIZE) -> ImageTk.PhotoImage:
    """Render a coloured circle with an initial letter as a PhotoImage."""
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    r, g, b = int(color[1:3], 16), int(color[3:5], 16), int(color[5:7], 16)
    draw.ellipse((0, 0, size - 1, size - 1), fill=(r, g, b, 255))
    # Draw the initial using PIL's default font (always available, no font files needed)
    try:
        from PIL import ImageFont
        font = ImageFont.truetype('segoeui.ttf', size=round(size * 0.45))
    except Exception:
        font = ImageFont.load_default()
    bbox = draw.textbbox((0, 0), initial, font=font)
    tw, th = bbox[2] - bbox[0], bbox[3] - bbox[1]
    draw.text(((size - tw) / 2 - bbox[0], (size - th) / 2 - bbox[1]),
              initial, font=font, fill=(255, 255, 255, 255))
    return ImageTk.PhotoImage(img)


def make_lock_photo(size: int = AVATAR_SIZE) -> ImageTk.PhotoImage:
    """Dark circle with a white padlock for Incognito/InPrivate."""
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    draw.ellipse((0, 0, size - 1, size - 1), fill=(59, 66, 97, 255))
    # Simple padlock: arc (shackle) + rectangle (body)
    m = size // 6
    sw = max(2, size // 12)
    lw = round(size * 0.36)
    lh = round(size * 0.28)
    lx = (size - lw) // 2
    ly = round(size * 0.42)
    # Shackle
    draw.arc((lx + sw, m, lx + lw - sw, ly + sw * 2),
             start=180, end=0, fill='white', width=sw)
    # Body
    draw.rounded_rectangle((lx, ly, lx + lw, ly + lh),
                            radius=max(2, size // 14), fill='white')
    return ImageTk.PhotoImage(img)


# ── Browser discovery ──────────────────────────────────────────────────────────

def _first_existing(*paths):
    return next((p for p in paths if os.path.exists(p)), None)

def _chrome_exe():
    return _first_existing(
        r'C:\Program Files\Google\Chrome\Application\chrome.exe',
        r'C:\Program Files (x86)\Google\Chrome\Application\chrome.exe',
    )

def _edge_exe():
    return _first_existing(
        r'C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe',
        r'C:\Program Files\Microsoft\Edge\Application\msedge.exe',
    )

def _read_profiles(user_data_path, exe, browser):
    base = Path(user_data_path)
    if not base.exists():
        return []

    names = {}
    local_state = base / 'Local State'
    if local_state.exists():
        try:
            data = json.loads(local_state.read_text(encoding='utf-8', errors='replace'))
            for d, info in data.get('profile', {}).get('info_cache', {}).items():
                names[d] = info.get('name') or d
        except Exception:
            pass

    profiles = []
    for item in sorted(base.iterdir()):
        if item.is_dir() and (item.name == 'Default' or item.name.startswith('Profile')):
            profiles.append({
                'browser':    browser,
                'name':       names.get(item.name, item.name),
                'dir':        item.name,
                'exe':        exe,
                'private':    False,
                'image_path': _find_profile_image(item),
            })
    return profiles

def discover_profiles():
    local = os.environ.get('LOCALAPPDATA', '')
    result = []

    chrome = _chrome_exe()
    if chrome:
        cp = _read_profiles(os.path.join(local, 'Google', 'Chrome', 'User Data'), chrome, 'chrome')
        result.extend(cp)
        if cp:
            result.append({'browser': 'chrome', 'name': 'Incognito', 'dir': None,
                           'exe': chrome, 'private': True, 'image_path': None})

    edge = _edge_exe()
    if edge:
        ep = _read_profiles(os.path.join(local, 'Microsoft', 'Edge', 'User Data'), edge, 'edge')
        result.extend(ep)
        if ep:
            result.append({'browser': 'edge', 'name': 'InPrivate', 'dir': None,
                           'exe': edge, 'private': True, 'image_path': None})

    return result

def launch(profile, url):
    cmd = [profile['exe']]
    if profile['private']:
        cmd.append('--incognito' if profile['browser'] == 'chrome' else '--inprivate')
    elif profile.get('dir'):
        cmd.append(f'--profile-directory={profile["dir"]}')
    cmd.append(url)
    subprocess.Popen(
        cmd,
        creationflags=subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP,
    )


def auto_launch_for_url(url, profiles) -> bool:
    """If a saved rule matches url and its browser/profile is still installed, launch it now."""
    rule = rules.find_matching_rule(url)
    profile = rules.resolve_profile(rule, profiles)
    if not profile:
        return False
    launch(profile, url)
    return True

# ── UI helpers ─────────────────────────────────────────────────────────────────

def _set_bg(widget, color):
    try:
        widget.configure(bg=color)
    except Exception:
        pass
    for child in widget.winfo_children():
        _set_bg(child, color)

def _all_widgets(root):
    yield root
    for child in root.winfo_children():
        yield from _all_widgets(child)


def build_avatar(profile, idx, images: list) -> ImageTk.PhotoImage:
    """Return a PhotoImage for the avatar; append to images to prevent GC."""
    if profile['private']:
        photo = make_lock_photo()
    elif profile.get('image_path'):
        try:
            photo = make_circular_photo(profile['image_path'])
        except Exception:
            photo = make_initial_photo(
                profile['name'][0].upper(),
                AVATAR_COLORS[idx % len(AVATAR_COLORS)],
            )
    else:
        photo = make_initial_photo(
            profile['name'][0].upper(),
            AVATAR_COLORS[idx % len(AVATAR_COLORS)],
        )
    images.append(photo)   # keep reference so tkinter doesn't GC it
    return photo


def make_card(parent, idx, profile, images: list, on_select,
              url=None, domain_rule=None, bg=BG_CARD):
    """
    Build and return a card Frame for one profile.
    on_select(profile) is called when the card is clicked.
    images must be a list held by the caller — PhotoImages are appended to prevent GC.

    url/domain_rule (optional) enable a right-click menu to set/unset this profile
    as the automatic default for the link's domain. domain_rule is the rule (or
    None) currently in effect for that domain, looked up once by the caller.
    """
    shortcut     = str(idx + 1) if idx < 9 else ('0' if idx == 9 else '')
    accent_color = BROWSER_ACCENT.get(profile['browser'], '#888')
    is_default   = rules.profile_matches_rule(profile, domain_rule)

    card = tk.Frame(parent, bg=bg, padx=10, pady=8, cursor='hand2')

    # Keyboard shortcut badge
    tk.Label(card, text=shortcut if shortcut else ' ',
             font=('Segoe UI', 9), bg=bg, fg=FG_DIM, width=2).pack(side='left', padx=(0, 8))

    # Avatar
    photo = build_avatar(profile, idx, images)
    av = tk.Canvas(card, width=AVATAR_SIZE, height=AVATAR_SIZE,
                   bg=bg, highlightthickness=0)
    av.pack(side='left', padx=(0, 10))
    av.create_image(AVATAR_SIZE // 2, AVATAR_SIZE // 2, image=photo, anchor='center')

    # Name + browser label
    info = tk.Frame(card, bg=bg)
    info.pack(side='left', fill='both', expand=True)
    tk.Label(info, text=profile['name'],
             font=('Segoe UI', 10, 'bold'), bg=bg, fg=FG, anchor='w').pack(anchor='w')
    browser_line = BROWSER_LABEL.get(profile['browser'], '')
    if is_default:
        browser_line = f'{browser_line}  ·  Default' if browser_line else 'Default'
    tk.Label(info, text=browser_line,
             font=('Segoe UI', 8), bg=bg, fg=FG_DIM, anchor='w').pack(anchor='w')

    # Browser accent bar
    tk.Frame(card, bg=accent_color, width=4).pack(side='right', fill='y', padx=(8, 0))

    def select(p=profile):
        on_select(p)

    def on_enter(_, c=card):
        _set_bg(c, BG_HOVER)

    def on_leave(_, c=card):
        _set_bg(c, bg)

    for w in _all_widgets(card):
        w.bind('<Button-1>', lambda _, p=profile: select(p))
        w.bind('<Enter>', on_enter)
        w.bind('<Leave>', on_leave)

    if url:
        domain = rules.domain_of(url)

        def set_default(d=domain, p=profile):
            rules.add_rule(d, 'domain', p)
            select(p)

        def remove_default(d=domain):
            rules.remove_rule(d, 'domain')

        def show_menu(event):
            menu = tk.Menu(card, tearoff=0, bg=BG_CARD, fg=FG,
                           activebackground=BG_HOVER, activeforeground=FG, bd=0)
            if domain:
                if is_default:
                    menu.add_command(label=f'Remove default for {domain}',
                                     command=remove_default)
                else:
                    menu.add_command(label=f'Always use {profile["name"]} for {domain}',
                                     command=set_default)
                menu.add_separator()
            menu.add_command(label='Manage defaults…',
                             command=lambda: open_rules_manager(card.winfo_toplevel(), url))
            menu.tk_popup(event.x_root, event.y_root)

        for w in _all_widgets(card):
            w.bind('<Button-3>', show_menu)

    return card


def open_rules_manager(parent, url=None):
    """Toplevel dialog to view, add, and remove default-browser rules."""
    profiles = discover_profiles()
    profile_options = [
        (f'{BROWSER_LABEL.get(p["browser"], p["browser"])} — {p["name"]}', p)
        for p in profiles if not p['private']
    ]
    type_label = {'domain': 'Domain', 'pattern': 'Pattern', 'url': 'Exact URL'}

    win = tk.Toplevel(parent)
    win.title('Default Browser Rules')
    win.configure(bg=BG)
    win.attributes('-topmost', True)
    win.resizable(False, False)

    tk.Label(win, text='Default browser rules', font=('Segoe UI', 11, 'bold'),
             bg=BG, fg=FG, padx=14).pack(anchor='w', pady=(12, 4))
    tk.Label(win, text='Matching links open automatically, skipping the picker.',
             font=('Segoe UI', 8), bg=BG, fg=FG_DIM, padx=14).pack(anchor='w', pady=(0, 8))
    tk.Frame(win, bg=BORDER, height=1).pack(fill='x')

    body = tk.Frame(win, bg=BG, padx=14, pady=8)
    body.pack(fill='both', expand=True)

    def profile_label_for_rule(rule):
        for label, p in profile_options:
            if rules.profile_matches_rule(p, rule):
                return label
        browser = BROWSER_LABEL.get(rule.get('browser'), rule.get('browser'))
        return f'{browser} (not installed)'

    def refresh():
        for w in body.winfo_children():
            w.destroy()
        current = rules.load_rules()
        if not current:
            tk.Label(body, text='No rules yet.', font=('Segoe UI', 9),
                     bg=BG, fg=FG_DIM).pack(anchor='w', pady=(0, 4))
        for r in current:
            row = tk.Frame(body, bg=BG_CARD, padx=8, pady=6)
            row.pack(fill='x', pady=2)
            tk.Label(row, text=f'[{type_label.get(r.get("type"), r.get("type"))}]',
                     font=('Segoe UI', 8), bg=BG_CARD, fg=FG_DIM).pack(side='left')
            tk.Label(row, text=r.get('pattern', ''), font=('Segoe UI', 9, 'bold'),
                     bg=BG_CARD, fg=FG, anchor='w').pack(side='left', padx=(6, 6))
            tk.Label(row, text=f'→ {profile_label_for_rule(r)}', font=('Segoe UI', 9),
                     bg=BG_CARD, fg=FG_DIM, anchor='w').pack(side='left', fill='x', expand=True)

            def remove(rule=r):
                rules.remove_rule(rule.get('pattern', ''), rule.get('type', ''))
                refresh()

            rm = tk.Label(row, text='✕', font=('Segoe UI', 9, 'bold'), bg=BG_CARD,
                          fg=FG_DIM, cursor='hand2')
            rm.pack(side='right')
            rm.bind('<Button-1>', lambda _, remove=remove: remove())

    refresh()
    tk.Frame(win, bg=BORDER, height=1).pack(fill='x')

    # ── Add rule form ───────────────────────────────────────────────
    form = tk.Frame(win, bg=BG, padx=14, pady=10)
    form.pack(fill='x')

    tk.Label(form, text='Add rule', font=('Segoe UI', 9, 'bold'),
             bg=BG, fg=FG).grid(row=0, column=0, columnspan=3, sticky='w', pady=(0, 6))

    pattern_var = tk.StringVar(value=rules.domain_of(url) if url else '')
    type_var    = tk.StringVar(value='domain')

    entry = tk.Entry(form, textvariable=pattern_var, width=34,
                     bg=BG_CARD, fg=FG, insertbackground=FG, relief='flat')
    entry.grid(row=1, column=0, columnspan=3, sticky='we', pady=(0, 6))

    for i, (val, text) in enumerate([('domain', 'Domain'),
                                      ('pattern', 'Pattern (*)'),
                                      ('url', 'Exact URL')]):
        tk.Radiobutton(form, text=text, variable=type_var, value=val,
                       bg=BG, fg=FG, selectcolor=BG_CARD,
                       activebackground=BG, activeforeground=FG,
                       font=('Segoe UI', 8)).grid(row=2, column=i, sticky='w')

    if profile_options:
        profile_var = tk.StringVar(value=profile_options[0][0])
        tk.OptionMenu(form, profile_var, *[label for label, _ in profile_options]).grid(
            row=3, column=0, columnspan=2, sticky='we', pady=(8, 0))

        def add():
            pattern = pattern_var.get().strip()
            if not pattern:
                return
            _, chosen = next(o for o in profile_options if o[0] == profile_var.get())
            rules.add_rule(pattern, type_var.get(), chosen)
            refresh()

        tk.Button(form, text='Add', command=add, bg=BG_CARD, fg=FG,
                 activebackground=BG_HOVER, relief='flat').grid(
            row=3, column=2, sticky='e', pady=(8, 0))

    form.grid_columnconfigure(0, weight=1)

    tk.Frame(win, bg=BORDER, height=1).pack(fill='x')
    close = tk.Label(win, text='Close', font=('Segoe UI', 9), bg=BG, fg=FG_DIM,
                     cursor='hand2', padx=14, pady=8)
    close.pack(anchor='e')
    close.bind('<Button-1>', lambda _: win.destroy())

    return win


# ── App ────────────────────────────────────────────────────────────────────────

class PickerApp:
    def __init__(self, url):
        self.url = url
        self.profiles = discover_profiles()
        self._done = False
        self._images = []   # keep PhotoImage refs alive
        self._auto_launched = False

        if auto_launch_for_url(url, self.profiles):
            self._auto_launched = True
            return

        root = tk.Tk()
        root.title('Browser Picker')
        root.configure(bg=BG)
        root.resizable(False, False)
        root.attributes('-topmost', True)
        root.withdraw()
        self.root = root

        self._build_ui()
        self._center()
        self._bind_keys()
        root.deiconify()
        root.focus_force()

    def _build_ui(self):
        root = self.root

        hdr = tk.Frame(root, bg=BG, padx=16, pady=14)
        hdr.pack(fill='x')
        tk.Label(hdr, text='Open link in…',
                 font=('Segoe UI', 12, 'bold'), bg=BG, fg=FG).pack(anchor='w')
        url_text = self.url if len(self.url) <= 60 else self.url[:57] + '…'
        tk.Label(hdr, text=url_text, font=('Segoe UI', 9),
                 bg=BG, fg=FG_DIM).pack(anchor='w', pady=(3, 0))

        tk.Frame(root, bg=BORDER, height=1).pack(fill='x')

        n = len(self.profiles)
        viewport_h = min(n, MAX_VISIBLE_CARDS) * CARD_HEIGHT + 16

        canvas = tk.Canvas(root, bg=BG, highlightthickness=0,
                           height=viewport_h, width=440)
        canvas.pack(fill='both', side='left', expand=True)

        if n > MAX_VISIBLE_CARDS:
            sb = tk.Scrollbar(root, orient='vertical', command=canvas.yview,
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

        tk.Frame(root, bg=BORDER, height=1).pack(fill='x')
        ftr = tk.Frame(root, bg=BG, padx=16, pady=8)
        ftr.pack(fill='x')
        hint = ('1–9 to pick  ·  scroll for more  ·  Esc to cancel'
                if n > 9 else '1–9 to pick  ·  Esc to cancel')
        tk.Label(ftr, text=hint, font=('Segoe UI', 8), bg=BG, fg=FG_DIM).pack(anchor='w', side='left')
        manage = tk.Label(ftr, text='Manage defaults…', font=('Segoe UI', 8),
                          bg=BG, fg=FG_DIM, cursor='hand2')
        manage.pack(anchor='e', side='right')
        manage.bind('<Button-1>', lambda _: open_rules_manager(root, self.url))

    def _select(self, profile):
        if not self._done:
            self._done = True
            launch(profile, self.url)
            self.root.destroy()

    def _bind_keys(self):
        self.root.bind('<Escape>', lambda _: self.root.destroy())
        for idx, profile in enumerate(self.profiles):
            key = str(idx + 1) if idx < 9 else ('0' if idx == 9 else None)
            if key:
                def _select(_, p=profile):
                    self._select(p)
                self.root.bind(key, _select)

    def _center(self):
        self.root.update_idletasks()
        w = max(460, self.root.winfo_reqwidth())
        h = self.root.winfo_reqheight()
        try:
            ml, mt, mr, mb = cursor_monitor_workarea()
        except Exception:
            ml, mt, mr, mb = 0, 0, self.root.winfo_screenwidth(), self.root.winfo_screenheight()
        x = ml + (mr - ml - w) // 2
        y = mt + (mb - mt - h) // 2
        self.root.geometry(f'{w}x{h}+{x}+{y}')

    def run(self):
        if self._auto_launched:
            return
        self.root.mainloop()


# ── Entry point ────────────────────────────────────────────────────────────────

def run_manager():
    """Show only the rules manager — no picker. Used by the Start Menu shortcut."""
    root = tk.Tk()
    root.withdraw()

    win = open_rules_manager(root)
    win.title('Browser Picker — Default Rules')
    win.protocol('WM_DELETE_WINDOW', win.destroy)
    win.bind('<Escape>', lambda _: win.destroy())
    # Closing the manager ends the process, since the root is only a hidden parent.
    win.bind('<Destroy>',
             lambda e: root.after_idle(root.destroy) if e.widget is win else None)

    win.update_idletasks()
    w, h = win.winfo_reqwidth(), win.winfo_reqheight()
    try:
        ml, mt, mr, mb = cursor_monitor_workarea()
    except Exception:
        ml, mt, mr, mb = 0, 0, win.winfo_screenwidth(), win.winfo_screenheight()
    win.geometry(f'{w}x{h}+{ml + (mr - ml - w) // 2}+{mt + (mb - mt - h) // 2}')
    win.focus_force()

    root.mainloop()


def main():
    args = sys.argv[1:]
    if args and args[0] in ('--manage', '-m'):
        run_manager()
        return
    url = args[0] if args else 'https://example.com/some/path?q=test'
    PickerApp(url).run()


if __name__ == '__main__':
    main()
