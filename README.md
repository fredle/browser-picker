# Browser Picker

Click a link, choose which Chrome or Edge profile opens it. Save a rule and that
site skips the picker from then on.

A **single 4.9 MB exe with no runtime dependency** and no background process.
Reaches a visible window in ~250 ms; a matched rule opens the browser and exits
in ~177 ms with no window at all.

The UI is a light theme — white cards on a near-white ground, hairline borders,
one blue accent — using the real Segoe UI so it matches the rest of Windows.

Rules are stored in `%LOCALAPPDATA%\BrowserPicker\rules.json`.

## Usage

```
browser_picker.exe <URL>          pick a profile for this link
browser_picker.exe --manage       rules manager + setup (also the default with no args)
browser_picker.exe --install      register with Windows, add the Start Menu entry
browser_picker.exe --uninstall    remove registration and Start Menu entry
browser_picker.exe --register     silent --install, for installers/scripting
browser_picker.exe --unregister   silent --uninstall, for installers/scripting
```

In the picker: `1`–`9` pick a profile, `Esc` cancels, **right-click a profile**
to make it the default for that domain, and *Manage defaults…* opens the rules
manager.

**Hold `Shift` while clicking a link** to ignore your saved rules and get the
picker anyway. Keep it held until the window appears (~250 ms): the modifier is
sampled with `GetAsyncKeyState` when the process starts, because the click was
handled by another application and its modifier state never reaches us. The key
is `bypass::KEY_NAME` / `bypass::held()` in `src/bypass.rs` if you want a
different one.

**Editing a rule:** *Edit* on any row loads it into the form; *Save* replaces it
**in place**. That matters because rules match first-to-last, so the delete-and-
retype it replaces would silently promote the rule to top priority. Editing also
absorbs a collision if you rename a rule onto an existing pattern.

Diagnostics (GUI subsystem, so redirect stdout to see them):

```
browser_picker.exe --status > out.txt         what Windows thinks the default browser is
browser_picker.exe --check <URL> > out.txt    which rule matches, and every discovered profile
```

## Default browser detection

The manage screen reports one of four states, read from
`HKCU\...\Shell\Associations\UrlAssociations\{http,https}\UserChoice` — the key
Windows actually consults on a link click:

- **Default** — holds both http and https.
- **Only partly set** — holds one scheme but not the other, so links split
  between browsers.
- **Another browser is default** — names the browser that currently holds it.
- **Not registered** — Windows doesn't list Browser Picker yet, so it can't be
  chosen. Offers a *Register now* button.

When it isn't the default, the screen shows the manual steps and a button that
opens Windows' Default Apps page (deep-linked to our entry on Windows 11 22H2+).
Windows deliberately allows no programmatic way to seize the default browser, so
the last step is always the user's. The screen re-reads the registry every
600 ms, so it updates itself when they come back from Settings.

## Build

Needs the MSVC Rust toolchain and a Windows SDK (for `rc.exe`, which embeds the
icon — the build degrades to no icon with a warning rather than failing).

```
cargo build --release        # target/release/browser_picker.exe
cargo test                   # rules matching + detection decision table
cargo fmt
```

`assets/browser_picker.ico` is generated art; `build.rs` embeds it along with
the version strings.

## Layout

| File | |
|---|---|
| `main.rs` | argument dispatch; the no-window fast path for a matched rule |
| `ui.rs` | picker and rules-manager screens, setup banner, theming |
| `profiles.rs` | Chrome/Edge discovery via `Local State` + profile dirs |
| `rules.rs` | rules.json load/save, fnmatch-compatible globbing |
| `default_browser.rs` | association detection, Default Apps deep link |
| `install.rs` | HKCU registration, Start Menu shortcut via `IShellLinkW` |
| `avatar.rs` | profile photo decode, scale, circular crop |
| `monitor.rs` | centring on the monitor under the cursor, DPI-aware |
| `theme.rs` | light palette: neutral surfaces, hairline borders, blue accent |
| `bypass.rs` | the hold-to-override modifier |

Debug builds accept `BP_FORCE_STATUS=default|partial|notdefault|notregistered`
to exercise the setup-guide states without touching the registry. It is compiled
out of release builds entirely.

## Distribution

Releases are built, signed (Azure Trusted Signing) and published automatically
by `.github/workflows/release.yml` on any `v*` tag. The installer is built with
Inno Setup (`installer/setup.iss`): a per-user install (no admin) that calls
`--register` after copying the exe and registers a proper uninstall entry that
runs `--unregister` first.

**Still to do:** a winget manifest — near-free now that releases are signed.
