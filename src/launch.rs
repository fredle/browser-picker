//! Launching a browser profile at a URL.

use crate::profiles::Profile;
use std::os::windows::process::CommandExt;

const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

pub fn launch(profile: &Profile, url: &str) {
    let mut cmd = std::process::Command::new(&profile.exe);
    if profile.private {
        cmd.arg(if profile.browser == "chrome" {
            "--incognito"
        } else {
            "--inprivate"
        });
    } else if let Some(dir) = &profile.dir {
        cmd.arg(format!("--profile-directory={dir}"));
    }
    cmd.arg(url);
    let _ = cmd
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn();
}
