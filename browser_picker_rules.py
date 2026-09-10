#!/usr/bin/env python3
"""
Browser Picker Rules — persist "always open this in profile X" rules
keyed by domain, URL pattern (glob), or exact URL.

Rules are stored as a JSON list at %LOCALAPPDATA%\\BrowserPicker\\rules.json.
Most-recently-added/updated rule is checked first.
"""

import json
import os
import fnmatch
from pathlib import Path
from urllib.parse import urlparse

RULES_DIR  = Path(os.environ.get('LOCALAPPDATA', '.')) / 'BrowserPicker'
RULES_PATH = RULES_DIR / 'rules.json'


def domain_of(url: str) -> str:
    try:
        host = urlparse(url).hostname
        return host.lower() if host else ''
    except Exception:
        return ''


def load_rules() -> list:
    try:
        data = json.loads(RULES_PATH.read_text(encoding='utf-8'))
        if isinstance(data, list):
            return data
    except Exception:
        pass
    return []


def save_rules(rules_list: list) -> None:
    RULES_DIR.mkdir(parents=True, exist_ok=True)
    RULES_PATH.write_text(json.dumps(rules_list, indent=2), encoding='utf-8')


def _profile_key(profile: dict) -> dict:
    return {
        'browser': profile['browser'],
        'dir':     profile.get('dir'),
        'private': profile.get('private', False),
    }


def add_rule(pattern: str, match_type: str, profile: dict) -> list:
    """Add/replace the rule for (pattern, match_type) and put it first (highest priority)."""
    current = [r for r in load_rules()
               if not (r.get('pattern') == pattern and r.get('type') == match_type)]
    current.insert(0, {'pattern': pattern, 'type': match_type, **_profile_key(profile)})
    save_rules(current)
    return current


def remove_rule(pattern: str, match_type: str) -> list:
    current = [r for r in load_rules()
               if not (r.get('pattern') == pattern and r.get('type') == match_type)]
    save_rules(current)
    return current


def find_domain_rule(domain: str, rules_list: list | None = None):
    """Exact-pattern lookup, used to reflect current UI state for a domain."""
    if not domain:
        return None
    if rules_list is None:
        rules_list = load_rules()
    for r in rules_list:
        if r.get('type') == 'domain' and r.get('pattern') == domain:
            return r
    return None


def _rule_matches_url(rule: dict, url: str, host: str) -> bool:
    t = rule.get('type')
    pattern = rule.get('pattern', '')
    if t == 'domain':
        return bool(host) and fnmatch.fnmatch(host, pattern)
    if t == 'pattern':
        return fnmatch.fnmatch(url, pattern)
    if t == 'url':
        return url == pattern or fnmatch.fnmatch(url, pattern)
    return False


def find_matching_rule(url: str, rules_list: list | None = None):
    """Return the first rule (in priority order) whose pattern matches url, or None."""
    if rules_list is None:
        rules_list = load_rules()
    host = domain_of(url)
    for rule in rules_list:
        if _rule_matches_url(rule, url, host):
            return rule
    return None


def profile_matches_rule(profile: dict, rule) -> bool:
    return (rule is not None
            and profile['browser'] == rule.get('browser')
            and profile.get('dir') == rule.get('dir')
            and profile.get('private', False) == rule.get('private', False))


def resolve_profile(rule, profiles: list):
    """Find the discovered profile a rule refers to (None if that browser/profile is gone)."""
    if rule is None:
        return None
    for p in profiles:
        if profile_matches_rule(p, rule):
            return p
    return None
