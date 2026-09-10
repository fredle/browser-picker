//! Browser Picker - choose which browser profile opens a link.
//!
//! Windows launches this exe directly as the http/https handler. A saved rule
//! is resolved and the browser launched without ever creating a window; only an
//! unmatched URL costs a GUI.
//!
//!   browser_picker.exe <URL>     pick a profile for this link
//!   browser_picker.exe --manage  rules manager and setup (also with no args)
//!   browser_picker.exe --install register with Windows, add Start Menu entry
//!   browser_picker.exe --uninstall

#![windows_subsystem = "windows"]

mod avatar;
mod bypass;
mod default_browser;
mod install;
mod launch;
mod monitor;
mod profiles;
mod rules;
mod theme;
mod ui;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        // Detection probe. GUI subsystem means stdout only lands somewhere if
        // the caller redirects it: browser_picker.exe --status > out.txt
        Some("--status") => {
            use std::io::Write;
            let _ = std::io::stdout().write_all(default_browser::report().as_bytes());
        }
        // Explain what would happen for a URL, without launching anything.
        Some("--check") => {
            use std::io::Write;
            let url = args.get(1).cloned().unwrap_or_default();
            let mut out = String::new();
            let list = rules::load();
            let discovered = profiles::discover();
            out.push_str(&format!(
                "url:    {url}
domain: {}
",
                rules::domain_of(&url)
            ));
            out.push_str(&format!(
                "bypass: hold {} at click time to skip rules (held now: {})
",
                bypass::KEY_NAME,
                bypass::held()
            ));
            match rules::find_matching(&url, &list) {
                Some(rule) => {
                    out.push_str(&format!(
                        "rule:   [{}] {} -> browser={} dir={:?} private={}
",
                        rule.kind, rule.pattern, rule.browser, rule.dir, rule.private
                    ));
                    match rules::resolve(Some(&rule), &discovered) {
                        Some(p) => out.push_str(&format!(
                            "action: launch {} without showing a window
        {}
",
                            p.label(),
                            p.exe.display()
                        )),
                        None => out.push_str(
                            "action: show picker (rule matched but that profile is gone)
",
                        ),
                    }
                }
                None => out.push_str(
                    "rule:   none
action: show picker
",
                ),
            }
            out.push_str(&format!(
                "
{} profiles discovered:
",
                discovered.len()
            ));
            for (i, p) in discovered.iter().enumerate() {
                out.push_str(&format!(
                    "  {}. {:<28} dir={:<12} private={:<5} photo={}
",
                    i + 1,
                    p.label(),
                    p.dir.clone().unwrap_or_else(|| "-".into()),
                    p.private,
                    p.image_path.is_some()
                ));
            }
            let _ = std::io::stdout().write_all(out.as_bytes());
        }
        Some("--register") => std::process::exit(install::run_register_quiet()),
        Some("--install") => install::run_install(),
        Some("--uninstall") => install::run_uninstall(),
        None | Some("--manage") | Some("-m") => {
            let _ = ui::run(ui::Screen::Rules, None);
        }
        Some(url) => {
            // Holding the bypass modifier means "ignore my rules, ask me".
            if !bypass::held() {
                // Fast path: a saved rule wins, so no window is ever created.
                let list = rules::load();
                if let Some(rule) = rules::find_matching(url, &list) {
                    let discovered = profiles::discover();
                    if let Some(profile) = rules::resolve(Some(&rule), &discovered) {
                        launch::launch(profile, url);
                        return;
                    }
                }
            }
            let _ = ui::run(ui::Screen::Picker, Some(url.to_string()));
        }
    }
}
