// SPDX-License-Identifier: Apache-2.0
//! SigmaOS-omarchy-chromium — Chromium-family browser manager.
//!
//! Detects installed Chromium browsers, manages profiles, builds launch
//! command lines (URL, incognito, flags), and tracks the default browser.
//! Deterministic core; exec/spawn is left to the caller/CLI.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BrowserKind { Chromium, Chrome, Brave, Edge, Firefox, Other }
impl BrowserKind {
    pub fn label(self) -> &'static str {
        match self { Self::Chromium=>"chromium",Self::Chrome=>"chrome",Self::Brave=>"brave",Self::Edge=>"edge",Self::Firefox=>"firefox",Self::Other=>"other" }
    }
}

#[derive(Debug, Clone)]
pub struct Browser {
    pub id: String,
    pub name: String,
    pub kind: BrowserKind,
    pub binary: PathBuf,
    pub profiles: Vec<Profile>,
    pub default_profile: String,
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,          // "Default", "Profile 1", ...
    pub path: PathBuf,
    pub guest: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserError {
    EmptyId,
    Duplicate(String),
    NoSuchBrowser(String),
    NoSuchProfile { browser: String, profile: String },
    NotFound(String),
}
impl fmt::Display for BrowserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => write!(f, "browser id must not be empty"),
            Self::Duplicate(id) => write!(f, "duplicate browser {id}"),
            Self::NoSuchBrowser(id) => write!(f, "no such browser {id}"),
            Self::NoSuchProfile { browser, profile } => write!(f, "{browser}: no profile {profile}"),
            Self::NotFound(b) => write!(f, "browser binary not found: {b}"),
        }
    }
}
impl std::error::Error for BrowserError {}

/// Privacy/UX flags Omarchy applies by default.
pub const DEFAULT_FLAGS: &[&str] = &[
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-session-crashed-bubble",
    "--hide-crash-restore-bubble",
    "--password-store=gnome-libsecret",
    "--enable-features=WaylandWindowDecorations",
];

pub struct OmarchyChromium {
    browsers: BTreeMap<String, Browser>,
    default_browser: Option<String>,
    extra_flags: Vec<String>,
}

impl OmarchyChromium {
    pub fn new() -> Self {
        Self { browsers: BTreeMap::new(), default_browser: None, extra_flags: Vec::new() }
    }

    /// Register a detected browser.
    pub fn register(&mut self, b: Browser) -> Result<(), BrowserError> {
        if b.id.trim().is_empty() { return Err(BrowserError::EmptyId); }
        if self.browsers.contains_key(&b.id) { return Err(BrowserError::Duplicate(b.id)); }
        if self.default_browser.is_none() { self.default_browser = Some(b.id.clone()); }
        self.browsers.insert(b.id.clone(), b);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Browser> { self.browsers.get(id) }
    pub fn browsers(&self) -> impl Iterator<Item = &Browser> { self.browsers.values() }
    pub fn count(&self) -> usize { self.browsers.len() }
    pub fn default(&self) -> Option<&Browser> { self.default_browser.as_ref().and_then(|id| self.browsers.get(id)) }

    /// Set the default browser id.
    pub fn set_default(&mut self, id: &str) -> Result<(), BrowserError> {
        if !self.browsers.contains_key(id) { return Err(BrowserError::NoSuchBrowser(id.into())); }
        self.default_browser = Some(id.into());
        Ok(())
    }

    pub fn add_flag(&mut self, flag: impl Into<String>) { self.extra_flags.push(flag.into()); }

    /// Build the launch command for a browser + optional URL + profile + incognito.
    /// Returns argv vector (binary first).
    pub fn launch_args(&self, browser_id: &str, url: Option<&str>, profile: Option<&str>, incognito: bool)
        -> Result<Vec<String>, BrowserError>
    {
        let b = self.browsers.get(browser_id).ok_or_else(|| BrowserError::NoSuchBrowser(browser_id.into()))?;
        let mut args = vec![b.binary.to_string_lossy().to_string()];
        for f in DEFAULT_FLAGS { args.push(f.to_string()); }
        for f in &self.extra_flags { args.push(f.clone()); }
        if incognito { args.push("--incognito".into()); }
        if let Some(p) = profile {
            if !b.profiles.iter().any(|pr| pr.name == p) {
                return Err(BrowserError::NoSuchProfile { browser: browser_id.into(), profile: p.into() });
            }
            args.push(format!("--profile-directory={p}"));
        }
        if let Some(u) = url { args.push(u.to_string()); }
        Ok(args)
    }

    /// Convenience: launch the default browser at a URL.
    pub fn open_url(&self, url: &str, incognito: bool) -> Result<Vec<String>, BrowserError> {
        let id = self.default_browser.clone().ok_or(BrowserError::NoSuchBrowser("<none>".into()))?;
        self.launch_args(&id, Some(url), None, incognito)
    }

    /// Detect common Chromium browsers on PATH (best-effort).
    pub fn detect() -> Vec<Browser> {
        let candidates = [
            ("chromium", "Chromium", BrowserKind::Chromium, "chromium"),
            ("chromium-browser", "Chromium", BrowserKind::Chromium, "chromium-browser"),
            ("google-chrome", "Google Chrome", BrowserKind::Chrome, "google-chrome"),
            ("brave", "Brave", BrowserKind::Brave, "brave-browser"),
            ("microsoft-edge", "Microsoft Edge", BrowserKind::Edge, "microsoft-edge"),
            ("firefox", "Firefox", BrowserKind::Firefox, "firefox"),
        ];
        let mut found = Vec::new();
        for (id, name, kind, bin) in candidates {
            // check PATH
            let in_path = std::env::var_os("PATH").map(|p| {
                std::env::split_paths(&p).any(|d| Path::new(&d).join(bin).exists())
            }).unwrap_or(false);
            let path = PathBuf::from(format!("/usr/bin/{bin}"));
            if in_path || path.exists() {
                let mut b = Browser {
                    id: id.into(), name: name.into(), kind,
                    binary: path, profiles: vec![Profile { name: "Default".into(), path: PathBuf::from("~/.config/chromium/Default"), guest: false }],
                    default_profile: "Default".into(),
                };
                // add a couple of common profiles if dirs exist
                for pn in ["Profile 1", "Profile 2"] {
                    let pp = PathBuf::from(format!("~/.config/chromium/{pn}"));
                    if pp.exists() { b.profiles.push(Profile { name: pn.into(), path: pp, guest: false }); }
                }
                found.push(b);
            }
        }
        found
    }

    pub fn status_line(&self) -> String {
        let d = self.default().map(|b| b.name.as_str()).unwrap_or("none");
        format!("chromium: {} browsers default={}", self.browsers.len(), d)
    }
}

impl Default for OmarchyChromium {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn browser(id: &str, bin: &str) -> Browser {
        Browser {
            id: id.into(), name: id.into(), kind: BrowserKind::Chromium,
            binary: PathBuf::from(bin),
            profiles: vec![
                Profile { name: "Default".into(), path: PathBuf::from("/tmp/Default"), guest: false },
                Profile { name: "Work".into(), path: PathBuf::from("/tmp/Work"), guest: false },
            ],
            default_profile: "Default".into(),
        }
    }

    #[test]
    fn register_and_default() {
        let mut m = OmarchyChromium::new();
        m.register(browser("chromium", "/usr/bin/chromium")).unwrap();
        m.register(browser("firefox", "/usr/bin/firefox")).unwrap();
        assert_eq!(m.count(), 2);
        assert_eq!(m.default().unwrap().id, "chromium"); // first registered
        m.set_default("firefox").unwrap();
        assert_eq!(m.default().unwrap().id, "firefox");
        assert!(m.set_default("nope").is_err());
        assert!(m.register(browser(" ", "/x")).is_err());
        assert!(m.register(browser("chromium", "/y")).is_err()); // dup
    }

    #[test]
    fn launch_args_basic() {
        let mut m = OmarchyChromium::new();
        m.register(browser("chromium", "/usr/bin/chromium")).unwrap();
        let args = m.launch_args("chromium", Some("https://example.com"), None, false).unwrap();
        assert_eq!(args[0], "/usr/bin/chromium");
        assert!(args.iter().any(|a| a == "--no-first-run"));
        assert!(args.iter().any(|a| a == "https://example.com"));
        assert!(!args.iter().any(|a| a == "--incognito"));
    }

    #[test]
    fn launch_args_profile_and_incognito() {
        let mut m = OmarchyChromium::new();
        m.register(browser("chromium", "/usr/bin/chromium")).unwrap();
        let args = m.launch_args("chromium", None, Some("Work"), true).unwrap();
        assert!(args.iter().any(|a| a == "--incognito"));
        assert!(args.iter().any(|a| a == "--profile-directory=Work"));
        assert!(m.launch_args("chromium", None, Some("Nope"), false).is_err());
    }

    #[test]
    fn open_url_uses_default() {
        let mut m = OmarchyChromium::new();
        m.register(browser("a", "/usr/bin/a")).unwrap();
        m.register(browser("b", "/usr/bin/b")).unwrap();
        m.set_default("b").unwrap();
        let args = m.open_url("https://x", true).unwrap();
        assert_eq!(args[0], "/usr/bin/b");
        assert!(args.iter().any(|a| a == "--incognito"));
    }

    #[test]
    fn extra_flags() {
        let mut m = OmarchyChromium::new();
        m.register(browser("c", "/usr/bin/c")).unwrap();
        m.add_flag("--force-dark-mode");
        let args = m.launch_args("c", None, None, false).unwrap();
        assert!(args.iter().any(|a| a == "--force-dark-mode"));
    }

    #[test]
    fn detect_runs() {
        let found = OmarchyChromium::detect();
        // on this system chromium/firefox may or may not exist; just ensure no panic
        for b in &found { assert!(!b.id.is_empty()); }
    }

    #[test]
    fn status() {
        let mut m = OmarchyChromium::new();
        assert!(m.status_line().contains("0 browsers"));
        m.register(browser("c", "/usr/bin/c")).unwrap();
        assert!(m.status_line().contains("default=c"));
    }
}
