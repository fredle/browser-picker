//! Persisted "always open this in profile X" rules.
//!
//! Reads and writes the same %LOCALAPPDATA%\BrowserPicker\rules.json the Python
//! version used, so existing rules keep working after the upgrade.

use crate::profiles::Profile;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Rule {
    pub pattern: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub browser: String,
    #[serde(default)]
    pub dir: Option<String>,
    #[serde(default)]
    pub private: bool,
}

pub fn rules_dir() -> PathBuf {
    PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into()))
        .join("BrowserPicker")
}

pub fn rules_path() -> PathBuf {
    rules_dir().join("rules.json")
}

pub fn load() -> Vec<Rule> {
    std::fs::read_to_string(rules_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(list: &[Rule]) {
    let _ = std::fs::create_dir_all(rules_dir());
    if let Ok(json) = serde_json::to_string_pretty(list) {
        let _ = std::fs::write(rules_path(), json);
    }
}

/// Add or replace the rule for (pattern, kind), at highest priority.
pub fn add(pattern: &str, kind: &str, profile: &Profile) -> Vec<Rule> {
    let mut list: Vec<Rule> = load()
        .into_iter()
        .filter(|r| !(r.pattern == pattern && r.kind == kind))
        .collect();
    list.insert(
        0,
        Rule {
            pattern: pattern.to_string(),
            kind: kind.to_string(),
            browser: profile.browser.to_string(),
            dir: profile.dir.clone(),
            private: profile.private,
        },
    );
    save(&list);
    list
}

/// Replace the rule identified by (old_pattern, old_kind) **in place**, keeping
/// its priority position. Rules match first-to-last, so editing one must not
/// silently promote it to the top the way `add` does.
///
/// Split from `update` so it can be tested without touching rules.json.
fn apply_update(mut list: Vec<Rule>, old_pattern: &str, old_kind: &str, new: Rule) -> Vec<Rule> {
    match list
        .iter()
        .position(|r| r.pattern == old_pattern && r.kind == old_kind)
    {
        Some(at) => {
            let (pattern, kind) = (new.pattern.clone(), new.kind.clone());
            list[at] = new;
            // If the edit collides with a different existing rule, that older
            // duplicate goes, matching `add`'s replace semantics.
            let mut i = 0;
            list.retain(|r| {
                let keep = i == at || !(r.pattern == pattern && r.kind == kind);
                i += 1;
                keep
            });
        }
        // The original is gone (deleted elsewhere) - behave like `add`.
        None => {
            list.retain(|r| !(r.pattern == new.pattern && r.kind == new.kind));
            list.insert(0, new);
        }
    }
    list
}

pub fn update(
    old_pattern: &str,
    old_kind: &str,
    pattern: &str,
    kind: &str,
    profile: &Profile,
) -> Vec<Rule> {
    let replacement = Rule {
        pattern: pattern.to_string(),
        kind: kind.to_string(),
        browser: profile.browser.clone(),
        dir: profile.dir.clone(),
        private: profile.private,
    };
    let list = apply_update(load(), old_pattern, old_kind, replacement);
    save(&list);
    list
}

pub fn remove(pattern: &str, kind: &str) -> Vec<Rule> {
    let list: Vec<Rule> = load()
        .into_iter()
        .filter(|r| !(r.pattern == pattern && r.kind == kind))
        .collect();
    save(&list);
    list
}

/// Host part of a URL, lowercased. Mirrors urlparse().hostname for normal URLs.
pub fn domain_of(url: &str) -> String {
    let s = match url.find("://") {
        Some(i) => &url[i + 3..],
        None => url,
    };
    let s = s.split(['/', '?', '#']).next().unwrap_or("");
    let s = match s.rfind('@') {
        Some(i) => &s[i + 1..],
        None => s,
    };
    let host = match s.strip_prefix('[') {
        Some(rest) => rest.split(']').next().unwrap_or(""),
        None => s.split(':').next().unwrap_or(""),
    };
    host.to_lowercase()
}

/// fnmatch-style glob. Case-insensitive, because Python's fnmatch normalises
/// case on Windows and existing rules were written against that behaviour.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    matches_from(&p, &t)
}

fn class_end(p: &[char]) -> Option<usize> {
    // p[0] is '['. A ']' immediately after (or after a negation) is a literal.
    let mut i = 1;
    if i < p.len() && (p[i] == '!' || p[i] == '^') {
        i += 1;
    }
    if i < p.len() && p[i] == ']' {
        i += 1;
    }
    while i < p.len() {
        if p[i] == ']' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn matches_from(p: &[char], t: &[char]) -> bool {
    if p.is_empty() {
        return t.is_empty();
    }
    match p[0] {
        '*' => (0..=t.len()).any(|i| matches_from(&p[1..], &t[i..])),
        '?' => !t.is_empty() && matches_from(&p[1..], &t[1..]),
        '[' => {
            if t.is_empty() {
                return false;
            }
            let Some(close) = class_end(p) else {
                return t[0] == '[' && matches_from(&p[1..], &t[1..]);
            };
            let mut set = &p[1..close];
            let mut negate = false;
            if !set.is_empty() && (set[0] == '!' || set[0] == '^') {
                negate = true;
                set = &set[1..];
            }
            let mut hit = false;
            let mut i = 0;
            while i < set.len() {
                if i + 2 < set.len() && set[i + 1] == '-' {
                    if t[0] >= set[i] && t[0] <= set[i + 2] {
                        hit = true;
                    }
                    i += 3;
                } else {
                    if t[0] == set[i] {
                        hit = true;
                    }
                    i += 1;
                }
            }
            if hit != negate {
                matches_from(&p[close + 1..], &t[1..])
            } else {
                false
            }
        }
        c => !t.is_empty() && t[0] == c && matches_from(&p[1..], &t[1..]),
    }
}

fn rule_matches_url(rule: &Rule, url: &str, host: &str) -> bool {
    match rule.kind.as_str() {
        "domain" => !host.is_empty() && glob_match(&rule.pattern, host),
        "pattern" => glob_match(&rule.pattern, url),
        "url" => url == rule.pattern || glob_match(&rule.pattern, url),
        _ => false,
    }
}

/// First rule, in priority order, whose pattern matches the URL.
pub fn find_matching(url: &str, list: &[Rule]) -> Option<Rule> {
    let host = domain_of(url);
    list.iter()
        .find(|r| rule_matches_url(r, url, &host))
        .cloned()
}

/// Exact-pattern lookup, used to reflect current UI state for a domain.
pub fn find_domain_rule(domain: &str, list: &[Rule]) -> Option<Rule> {
    if domain.is_empty() {
        return None;
    }
    list.iter()
        .find(|r| r.kind == "domain" && r.pattern == domain)
        .cloned()
}

pub fn profile_matches(profile: &Profile, rule: Option<&Rule>) -> bool {
    match rule {
        Some(r) => {
            profile.browser == r.browser && profile.dir == r.dir && profile.private == r.private
        }
        None => false,
    }
}

/// The discovered profile a rule refers to, or None if that profile is gone.
pub fn resolve<'a>(rule: Option<&Rule>, profiles: &'a [Profile]) -> Option<&'a Profile> {
    let rule = rule?;
    profiles.iter().find(|p| profile_matches(p, Some(rule)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domains() {
        assert_eq!(domain_of("https://github.com/foo/bar"), "github.com");
        assert_eq!(domain_of("http://Example.COM"), "example.com");
        assert_eq!(domain_of("https://user:pw@host.dev:8443/x"), "host.dev");
        assert_eq!(domain_of("https://[::1]:8080/x"), "::1");
        assert_eq!(domain_of("github.com/foo"), "github.com");
        assert_eq!(domain_of(""), "");
    }

    #[test]
    fn globs() {
        assert!(glob_match("github.com", "github.com"));
        assert!(!glob_match("github.com", "gist.github.com"));
        assert!(glob_match("*.github.com", "gist.github.com"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match(
            "*://*.atlassian.net/browse/*",
            "https://x.atlassian.net/browse/AB-1"
        ));
        assert!(!glob_match(
            "*://*.atlassian.net/browse/*",
            "https://x.atlassian.net/wiki/AB-1"
        ));
        // case-insensitive, matching Python's fnmatch on Windows
        assert!(glob_match("GitHub.com", "github.com"));
        // ? matches exactly one character
        assert!(glob_match("a?c", "abc"));
        assert!(!glob_match("a?c", "ac"));
        // character classes, including negation and ranges
        assert!(glob_match("item[0-9]", "item7"));
        assert!(!glob_match("item[0-9]", "itemx"));
        assert!(glob_match("item[!0-9]", "itemx"));
        assert!(!glob_match("item[!0-9]", "item7"));
        // an unclosed bracket is a literal
        assert!(glob_match("a[bc", "a[bc"));
    }

    #[test]
    fn rule_priority_and_kinds() {
        let list = vec![
            Rule {
                pattern: "*.example.com".into(),
                kind: "domain".into(),
                browser: "chrome".into(),
                dir: Some("Profile 1".into()),
                private: false,
            },
            Rule {
                pattern: "*/secret/*".into(),
                kind: "pattern".into(),
                browser: "edge".into(),
                dir: Some("Default".into()),
                private: false,
            },
        ];
        // first match in list order wins
        let hit = find_matching("https://a.example.com/secret/x", &list).unwrap();
        assert_eq!(hit.browser, "chrome");
        let hit = find_matching("https://other.net/secret/x", &list).unwrap();
        assert_eq!(hit.browser, "edge");
        assert!(find_matching("https://nope.net/", &list).is_none());
        // domain rules must not match against the whole URL
        let only_domain = vec![Rule {
            pattern: "github.com".into(),
            kind: "domain".into(),
            browser: "chrome".into(),
            dir: None,
            private: true,
        }];
        assert!(find_matching("https://github.com/x", &only_domain).is_some());
        assert!(find_matching("https://evil.net/?q=github.com", &only_domain).is_none());
    }

    fn rule(pattern: &str, kind: &str, browser: &str, dir: Option<&str>) -> Rule {
        Rule {
            pattern: pattern.into(),
            kind: kind.into(),
            browser: browser.into(),
            dir: dir.map(Into::into),
            private: false,
        }
    }

    #[test]
    fn edit_keeps_priority_position() {
        let list = vec![
            rule("a.com", "domain", "chrome", Some("Default")),
            rule("b.com", "domain", "chrome", Some("Default")),
            rule("c.com", "domain", "chrome", Some("Default")),
        ];
        let out = apply_update(
            list,
            "b.com",
            "domain",
            rule("b2.com", "domain", "edge", Some("Profile 1")),
        );
        let patterns: Vec<&str> = out.iter().map(|r| r.pattern.as_str()).collect();
        assert_eq!(
            patterns,
            ["a.com", "b2.com", "c.com"],
            "must not jump to front"
        );
        assert_eq!(out[1].browser, "edge");
        assert_eq!(out[1].dir.as_deref(), Some("Profile 1"));
    }

    #[test]
    fn edit_can_change_kind_in_place() {
        let list = vec![
            rule("x.com", "domain", "chrome", Some("Default")),
            rule("y.com", "domain", "chrome", Some("Default")),
        ];
        let out = apply_update(
            list,
            "y.com",
            "domain",
            rule("*://y.com/app/*", "pattern", "chrome", Some("Default")),
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[1].kind, "pattern");
        assert_eq!(out[1].pattern, "*://y.com/app/*");
    }

    #[test]
    fn edit_collapses_a_collision() {
        // Editing "a.com" into "c.com" should absorb the existing c.com rule,
        // keeping a single rule at the edited rule's original position.
        let list = vec![
            rule("a.com", "domain", "chrome", Some("Default")),
            rule("b.com", "domain", "chrome", Some("Default")),
            rule("c.com", "domain", "edge", Some("Default")),
        ];
        let out = apply_update(
            list,
            "a.com",
            "domain",
            rule("c.com", "domain", "chrome", Some("Profile 3")),
        );
        let patterns: Vec<&str> = out.iter().map(|r| r.pattern.as_str()).collect();
        assert_eq!(patterns, ["c.com", "b.com"]);
        assert_eq!(out[0].dir.as_deref(), Some("Profile 3"));
    }

    #[test]
    fn edit_of_a_vanished_rule_falls_back_to_add() {
        let list = vec![rule("keep.com", "domain", "chrome", Some("Default"))];
        let out = apply_update(
            list,
            "gone.com",
            "domain",
            rule("new.com", "domain", "chrome", Some("Default")),
        );
        let patterns: Vec<&str> = out.iter().map(|r| r.pattern.as_str()).collect();
        assert_eq!(patterns, ["new.com", "keep.com"]);
    }

    #[test]
    fn exact_url_kind() {
        let list = vec![Rule {
            pattern: "https://a.com/x".into(),
            kind: "url".into(),
            browser: "edge".into(),
            dir: Some("Default".into()),
            private: false,
        }];
        assert!(find_matching("https://a.com/x", &list).is_some());
        assert!(find_matching("https://a.com/x/y", &list).is_none());
    }
}
