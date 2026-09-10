//! Detecting whether Browser Picker actually holds the http/https association,
//! and getting the user to the one place Windows lets them change it.

use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, PCWSTR};
use winreg::RegKey;
use winreg::enums::*;

pub const PROG_ID: &str = "BrowserPickerURL";
pub const APP_NAME: &str = "BrowserPicker";
pub const DISPLAY_NAME: &str = "Browser Picker";

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Status {
    /// Holds both http and https.
    Default,
    /// Holds one scheme but not the other - links will split between browsers.
    Partial,
    /// Registered with Windows, but another browser holds the associations.
    NotDefault,
    /// Windows doesn't know about us, so we aren't even listed in Default Apps.
    NotRegistered,
}

impl Status {
    pub fn is_ok(self) -> bool {
        self == Status::Default
    }
}

/// The ProgId the user has actually chosen for a scheme. This key is the
/// authoritative answer - it is what Windows consults on a link click.
fn user_choice(scheme: &str) -> Option<String> {
    let path = format!(
        r"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\{scheme}\UserChoice"
    );
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(path)
        .ok()?
        .get_value::<String, _>("ProgId")
        .ok()
}

/// Whether our ProgId and RegisteredApplications entry exist. Without these,
/// Browser Picker cannot appear in the Default Apps list at all.
pub fn is_registered() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(format!(r"Software\Classes\{PROG_ID}\shell\open\command"))
        .is_ok()
        && hkcu
            .open_subkey(r"Software\RegisteredApplications")
            .and_then(|k| k.get_value::<String, _>(APP_NAME))
            .is_ok()
}

/// Pure decision table, split out from the registry reads so it can be tested.
fn classify(registered: bool, http: Option<&str>, https: Option<&str>) -> Status {
    if !registered {
        return Status::NotRegistered;
    }
    match (http == Some(PROG_ID), https == Some(PROG_ID)) {
        (true, true) => Status::Default,
        (false, false) => Status::NotDefault,
        _ => Status::Partial,
    }
}

pub fn status() -> Status {
    // Debug builds only: lets the setup-guide branches be exercised without
    // touching the real registry. Never compiled into a release binary.
    #[cfg(debug_assertions)]
    if let Ok(forced) = std::env::var("BP_FORCE_STATUS") {
        return match forced.as_str() {
            "default" => Status::Default,
            "partial" => Status::Partial,
            "notdefault" => Status::NotDefault,
            _ => Status::NotRegistered,
        };
    }
    classify(
        is_registered(),
        user_choice("http").as_deref(),
        user_choice("https").as_deref(),
    )
}

/// Human-readable detection report, for `--status`.
pub fn report() -> String {
    let http = user_choice("http").unwrap_or_else(|| "(unset)".into());
    let https = user_choice("https").unwrap_or_else(|| "(unset)".into());
    format!(
        "registered:      {}
http  UserChoice: {http}
https UserChoice: {https}
current default: {}
status:          {:?}
missing:         {:?}
",
        is_registered(),
        current_default_name().unwrap_or_else(|| "(unknown)".into()),
        status(),
        missing_schemes(),
    )
}

/// Friendly name of whatever currently owns https, for display in the UI.
pub fn current_default_name() -> Option<String> {
    let prog_id = user_choice("https")?;
    if prog_id == PROG_ID {
        return Some(DISPLAY_NAME.to_string());
    }
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let friendly = hkcr
        .open_subkey(format!(r"{prog_id}\Application"))
        .and_then(|k| k.get_value::<String, _>("ApplicationName"))
        .ok()
        .or_else(|| {
            hkcr.open_subkey(&prog_id)
                .and_then(|k| k.get_value::<String, _>(""))
                .ok()
        })
        .filter(|s| !s.trim().is_empty());
    Some(friendly.unwrap_or(prog_id))
}

/// Which of http/https we do not hold, for precise guidance.
pub fn missing_schemes() -> Vec<&'static str> {
    ["http", "https"]
        .into_iter()
        .filter(|s| user_choice(s).as_deref() != Some(PROG_ID))
        .collect()
}

fn shell_open(target: &str) {
    unsafe {
        ShellExecuteW(
            None,
            &HSTRING::from("open"),
            &HSTRING::from(target),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

/// Open the Windows Default Apps page. Windows 11 22H2+ honours the query
/// string and jumps straight to our entry; older builds show the main page.
pub fn open_default_apps_settings() {
    shell_open("ms-settings:defaultapps?registeredAppUser=Browser%20Picker");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_four_branches() {
        let other = Some("ChromeHTML");
        let ours = Some(PROG_ID);
        assert_eq!(classify(false, ours, ours), Status::NotRegistered);
        assert_eq!(classify(true, ours, ours), Status::Default);
        assert_eq!(classify(true, other, other), Status::NotDefault);
        assert_eq!(classify(true, None, None), Status::NotDefault);
        assert_eq!(classify(true, ours, other), Status::Partial);
        assert_eq!(classify(true, other, ours), Status::Partial);
        assert_eq!(classify(true, ours, None), Status::Partial);
        assert!(classify(true, ours, ours).is_ok());
        assert!(!classify(true, ours, other).is_ok());
    }
}
