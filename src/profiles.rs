//! Chrome / Edge profile discovery.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Profile {
    pub browser: String,
    pub name: String,
    pub dir: Option<String>,
    pub exe: PathBuf,
    pub private: bool,
    pub image_path: Option<PathBuf>,
}

impl Profile {
    /// Label used in the rules manager, e.g. "Chrome — Personal".
    pub fn label(&self) -> String {
        format!(
            "{} \u{2014} {}",
            crate::theme::browser_label(&self.browser),
            self.name
        )
    }
}

fn first_existing(paths: &[&str]) -> Option<PathBuf> {
    paths.iter().map(PathBuf::from).find(|p| p.exists())
}

fn chrome_exe() -> Option<PathBuf> {
    first_existing(&[
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ])
}

fn edge_exe() -> Option<PathBuf> {
    first_existing(&[
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ])
}

/// Best available profile photo, in order of preference.
fn find_profile_image(dir: &Path) -> Option<PathBuf> {
    [
        "Google Profile Picture.png",
        "Edge Profile Picture.png",
        "Google Profile.ico",
        "Edge Profile.ico",
    ]
    .iter()
    .map(|n| dir.join(n))
    .find(|p| p.exists())
}

fn read_profiles(user_data: &Path, exe: &Path, browser: &str) -> Vec<Profile> {
    if !user_data.exists() {
        return Vec::new();
    }

    // Display names live in Local State; fall back to the directory name.
    let mut names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Ok(text) = std::fs::read_to_string(user_data.join("Local State")) {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(cache) = data
                .get("profile")
                .and_then(|p| p.get("info_cache"))
                .and_then(|c| c.as_object())
            {
                for (dir, info) in cache {
                    let name = info
                        .get("name")
                        .and_then(|n| n.as_str())
                        .filter(|s| !s.is_empty())
                        .unwrap_or(dir);
                    names.insert(dir.clone(), name.to_string());
                }
            }
        }
    }

    let Ok(entries) = std::fs::read_dir(user_data) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            if !p.is_dir() {
                return false;
            }
            let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                return false;
            };
            name == "Default" || name.starts_with("Profile")
        })
        .collect();
    dirs.sort();

    dirs.into_iter()
        .map(|p| {
            let dir = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            Profile {
                browser: browser.to_string(),
                name: names.get(&dir).cloned().unwrap_or_else(|| dir.clone()),
                image_path: find_profile_image(&p),
                dir: Some(dir),
                exe: exe.to_path_buf(),
                private: false,
            }
        })
        .collect()
}

pub fn discover() -> Vec<Profile> {
    let local = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default());
    let mut result = Vec::new();

    for (browser, exe, user_data, private_name) in [
        (
            "chrome",
            chrome_exe(),
            local.join(r"Google\Chrome\User Data"),
            "Incognito",
        ),
        (
            "edge",
            edge_exe(),
            local.join(r"Microsoft\Edge\User Data"),
            "InPrivate",
        ),
    ] {
        let Some(exe) = exe else { continue };
        let found = read_profiles(&user_data, &exe, browser);
        if found.is_empty() {
            continue;
        }
        result.extend(found);
        result.push(Profile {
            browser: browser.to_string(),
            name: private_name.to_string(),
            dir: None,
            exe,
            private: true,
            image_path: None,
        });
    }

    result
}
