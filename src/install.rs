//! Registering as a Windows browser.
//!
//! Everything is per-user (HKCU), so no admin rights are needed. There is no
//! daemon and no autostart entry - Windows launches the exe directly. The
//! Start Menu shortcut is Velopack's, not ours (see `src/update.rs`): it
//! already points at the exe with no arguments, which defaults to the rules
//! manager, and Velopack keeps it pointed at the current version across
//! updates - a shortcut we made ourselves would go stale after the first
//! self-update.

use crate::default_browser::{APP_NAME, DISPLAY_NAME, PROG_ID};
use std::path::PathBuf;
use windows::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MessageBoxW,
};
use windows::core::HSTRING;
use winreg::RegKey;
use winreg::enums::*;

fn exe_path() -> PathBuf {
    std::env::current_exe().unwrap_or_default()
}

fn open_command() -> String {
    format!("\"{}\" \"%1\"", exe_path().display())
}

pub fn register() -> std::io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let caps = format!(r"Software\Clients\StartMenuInternet\{APP_NAME}\Capabilities");

    let (k, _) = hkcu.create_subkey(format!(r"Software\Classes\{PROG_ID}"))?;
    k.set_value("", &"Browser Picker URL")?;
    k.set_value("URL Protocol", &"")?;

    let (k, _) = hkcu.create_subkey(format!(r"Software\Classes\{PROG_ID}\shell\open\command"))?;
    k.set_value("", &open_command())?;

    let (k, _) = hkcu.create_subkey(format!(r"Software\Classes\{PROG_ID}\DefaultIcon"))?;
    k.set_value("", &format!("{},0", exe_path().display()))?;

    let (k, _) = hkcu.create_subkey(format!(r"Software\Clients\StartMenuInternet\{APP_NAME}"))?;
    k.set_value("", &DISPLAY_NAME)?;

    let (k, _) = hkcu.create_subkey(format!(
        r"Software\Clients\StartMenuInternet\{APP_NAME}\shell\open\command"
    ))?;
    k.set_value("", &open_command())?;

    let (k, _) = hkcu.create_subkey(&caps)?;
    k.set_value("ApplicationName", &DISPLAY_NAME)?;
    k.set_value(
        "ApplicationDescription",
        &"Choose which browser profile to open links in",
    )?;
    k.set_value("ApplicationIcon", &format!("{},0", exe_path().display()))?;

    let (k, _) = hkcu.create_subkey(format!(r"{caps}\URLAssociations"))?;
    k.set_value("http", &PROG_ID)?;
    k.set_value("https", &PROG_ID)?;

    let (k, _) = hkcu.create_subkey(r"Software\RegisteredApplications")?;
    k.set_value(APP_NAME, &caps)?;

    Ok(())
}

pub fn unregister() -> std::io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(format!(r"Software\Classes\{PROG_ID}"));
    let _ = hkcu.delete_subkey_all(format!(r"Software\Clients\StartMenuInternet\{APP_NAME}"));
    if let Ok(k) = hkcu.open_subkey_with_flags(r"Software\RegisteredApplications", KEY_SET_VALUE) {
        let _ = k.delete_value(APP_NAME);
    }
    Ok(())
}

fn message(text: &str, error: bool) {
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(text),
            &HSTRING::from(DISPLAY_NAME),
            MB_OK
                | if error {
                    MB_ICONERROR
                } else {
                    MB_ICONINFORMATION
                },
        );
    }
}

/// Silent registration for unattended use (scripting, Velopack's own install
/// hook): no dialog, no Settings page. Exit code reports success.
pub fn run_register_quiet() -> i32 {
    match register() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("register failed: {e}");
            1
        }
    }
}

pub fn run_install() {
    if let Err(e) = register() {
        message(&format!("Could not register with Windows:\n{e}"), true);
        return;
    }
    message(
        "Browser Picker is registered with Windows.\n\n\
         Last step: Windows only lets you choose a default browser by hand. \
         Click OK to open Default apps, then set Browser Picker for HTTP and HTTPS.",
        false,
    );
    crate::default_browser::open_default_apps_settings();
}

/// Silent unregistration for unattended use (scripting, Velopack's own
/// uninstall hook): no dialog, no Settings page. Exit code reports success.
pub fn run_unregister_quiet() -> i32 {
    match unregister() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("unregister failed: {e}");
            1
        }
    }
}

pub fn run_uninstall() {
    let _ = unregister();
    message(
        "Browser Picker has been unregistered.\n\n\
         Remember to pick a different default browser in Settings > Apps > Default apps.",
        false,
    );
}
