//! Self-update via GitHub Releases, using Velopack.
//!
//! Two halves: `install_hooks()` wires Velopack's own install/uninstall/
//! update lifecycle to our Windows registration (`crate::install`), and
//! `check()` / `install_and_restart()` back the "check for updates" UI in the
//! rules manager. Both use the same source, so both only work on a build
//! that was actually installed by Velopack - a `cargo run` dev build has no
//! app id or install root for `UpdateManager` to find, and `check()` simply
//! reports that as an error rather than panicking.

use velopack::sources::GithubSource;
use velopack::{UpdateCheck, UpdateInfo, UpdateManager, VelopackApp};

const REPO_URL: &str = "https://github.com/fredle/browser-picker";

/// Wires Velopack's install/uninstall/update lifecycle to our own Windows
/// registration, and prompts to set the default browser on first run. Must
/// run before anything else in `main` - Velopack may terminate or restart
/// the process to carry out one of these steps.
pub fn run_app_hooks() {
    VelopackApp::build()
        .on_after_install_fast_callback(|_v| {
            let _ = crate::install::register();
        })
        .on_before_uninstall_fast_callback(|_v| {
            let _ = crate::install::unregister();
        })
        .on_after_update_fast_callback(|_v| {
            // The registered open-command embeds the exe's own path, which
            // moves to a new versioned folder on every update.
            let _ = crate::install::register();
        })
        .on_first_run(|_v| {
            crate::default_browser::open_default_apps_settings();
        })
        .run();
}

fn manager() -> Result<UpdateManager, String> {
    let source = GithubSource::new(REPO_URL, None, false);
    UpdateManager::new(source, None, None).map_err(|e| e.to_string())
}

/// Checks GitHub Releases for a newer version. `Ok(None)` covers both "up to
/// date" and "not an installed Velopack build" alike - there is nothing
/// actionable to show the user either way.
pub fn check() -> Option<UpdateInfo> {
    let um = manager().ok()?;
    match um.check_for_updates().ok()? {
        UpdateCheck::UpdateAvailable(info) => Some(*info),
        _ => None,
    }
}

/// Downloads the update and relaunches into it. Does not return on success -
/// the whole point is that the running process gets replaced.
pub fn install_and_restart(info: &UpdateInfo) -> Result<(), String> {
    let um = manager()?;
    um.download_updates(info, None).map_err(|e| e.to_string())?;
    um.apply_updates_and_restart(&info.TargetFullRelease)
        .map_err(|e| e.to_string())
}
