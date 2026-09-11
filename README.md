# Browser Picker

Click a link, choose which Chrome or Edge profile opens it. Save a rule and that
site skips the picker from then on.

A **single 4.9 MB exe with no runtime dependency** and no background process.
Reaches a visible window in ~250 ms; a matched rule opens the browser and exits
in ~177 ms with no window at all.

The UI is a light theme — white cards on a near-white ground, hairline borders,
one blue accent — using the real Segoe UI so it matches the rest of Windows.

Rules are stored in `%LOCALAPPDATA%\BrowserPicker\rules.json`. Installed
builds update themselves in the background from GitHub Releases (Velopack) -
see *Distribution* below.

## Usage

```
browser_picker.exe <URL>          pick a profile for this link
browser_picker.exe --manage       rules manager + setup (also the default with no args)
browser_picker.exe --install      register with Windows (dev/manual use - an installed
browser_picker.exe --uninstall    build registers itself via Velopack's own hooks)
browser_picker.exe --register     silent --install
browser_picker.exe --unregister   silent --uninstall
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

**Protected links (Mimecast, SafeLinks, Proofpoint, ...):** off by default, a
checkbox in *Manage defaults…* resolves these to the real destination before
showing the picker, so both the display and any rule you save are keyed off
the actual site rather than the wrapper. It's opt-in because resolving one
sends a request to the wrapper's server, which registers as a click against
the original link.

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
| `install.rs` | HKCU browser registration (Start Menu shortcut is Velopack's) |
| `avatar.rs` | profile photo decode, scale, circular crop |
| `monitor.rs` | centring on the monitor under the cursor, DPI-aware |
| `theme.rs` | light palette: neutral surfaces, hairline borders, blue accent |
| `bypass.rs` | the hold-to-override modifier |
| `unwrap.rs` | resolves known safe-link wrappers (Mimecast, SafeLinks, ...) to their real destination |
| `settings.rs` | settings.json load/save (currently just the unwrap opt-in) |
| `update.rs` | Velopack app hooks + GitHub Releases self-update |

Debug builds accept `BP_FORCE_STATUS=default|partial|notdefault|notregistered`
to exercise the setup-guide states without touching the registry. It is compiled
out of release builds entirely.

## Distribution

Releases are built, signed (Azure Trusted Signing) and published automatically
by `.github/workflows/release.yml` on any `v*` tag, using
[Velopack](https://velopack.io) (`vpk pack` / `vpk upload github`): a per-user
install (no admin) with its own `Setup.exe`, Start Menu shortcut, and
uninstall entry.

Velopack also gives installed builds self-update: `update.rs` wires its
install/uninstall/update lifecycle hooks to our own Windows browser
registration (`install::register`/`unregister`, since Velopack doesn't know
about that), and the rules manager checks GitHub Releases for a newer version
in the background, offering an *Update & restart* button when one is found.
None of this applies to a `cargo build` dev binary - there's no installed app
id or root for `UpdateManager` to find, so the check just quietly finds
nothing.

**Migration note:** existing installs from before Velopack (Inno Setup,
tagged v0.1.4 and earlier) won't self-update into the new layout - those
users need to grab the new installer once by hand.

**Still to do:** a winget manifest — near-free now that releases are signed.
