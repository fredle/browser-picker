//! Registering as a Windows browser, and the Start Menu entry.
//!
//! Everything is per-user (HKCU), so no admin rights are needed. There is no
//! daemon and no autostart entry - Windows launches the exe directly.

use crate::default_browser::{APP_NAME, DISPLAY_NAME, PROG_ID};
use std::path::PathBuf;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, IPersistFile,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
use windows::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MessageBoxW,
};
use windows::core::{HSTRING, Interface};
use winreg::RegKey;
use winreg::enums::*;

const SHORTCUT_NAME: &str = "Browser Picker.lnk";

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
    let _ = std::fs::remove_file(shortcut_path());
    Ok(())
}

fn shortcut_path() -> PathBuf {
    PathBuf::from(std::env::var("APPDATA").unwrap_or_default())
        .join(r"Microsoft\Windows\Start Menu\Programs")
        .join(SHORTCUT_NAME)
}

pub fn create_start_menu_shortcut() -> windows::core::Result<PathBuf> {
    let lnk = shortcut_path();
    if let Some(parent) = lnk.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let exe = exe_path();
    let workdir = exe.parent().map(|p| p.to_path_buf()).unwrap_or_default();

    unsafe {
        // Already-initialised is not an error for our purposes.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        link.SetPath(&HSTRING::from(exe.to_string_lossy().as_ref()))?;
        link.SetArguments(&HSTRING::from("--manage"))?;
        link.SetDescription(&HSTRING::from(
            "Manage which browser profile opens which sites",
        ))?;
        link.SetWorkingDirectory(&HSTRING::from(workdir.to_string_lossy().as_ref()))?;

        let file: IPersistFile = link.cast()?;
        file.Save(&HSTRING::from(lnk.to_string_lossy().as_ref()), true)?;
    }

    Ok(lnk)
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

/// Silent registration for unattended use (installers, scripting): no dialog,
/// no Settings page. Exit code reports success.
pub fn run_register_quiet() -> i32 {
    if let Err(e) = register() {
        eprintln!("register failed: {e}");
        return 1;
    }
    match create_start_menu_shortcut() {
        Ok(p) => println!("registered; shortcut: {}", p.display()),
        Err(e) => {
            println!("registered; shortcut failed: {e}");
            return 2;
        }
    }
    0
}

pub fn run_install() {
    if let Err(e) = register() {
        message(&format!("Could not register with Windows:\n{e}"), true);
        return;
    }
    let shortcut = match create_start_menu_shortcut() {
        Ok(p) => format!("Start Menu shortcut created:\n{}", p.display()),
        Err(e) => format!("Start Menu shortcut could not be created:\n{e}"),
    };
    message(
        &format!(
            "Browser Picker is registered with Windows.\n\n{shortcut}\n\n\
             Last step: Windows only lets you choose a default browser by hand. \
             Click OK to open Default apps, then set Browser Picker for HTTP and HTTPS."
        ),
        false,
    );
    crate::default_browser::open_default_apps_settings();
}

pub fn run_uninstall() {
    let _ = unregister();
    message(
        "Browser Picker has been unregistered and its Start Menu entry removed.\n\n\
         Remember to pick a different default browser in Settings > Apps > Default apps.",
        false,
    );
}
