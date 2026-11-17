//! SigmaOS-omarchy-chromium — browser manager CLI.
//!
//! Usage:
//!   omarchy-chromium detect          (scan PATH for browsers)
//!   omarchy-chromium list
//!   omarchy-chromium default [id]
//!   omarchy-chromium open <url> [--private] [--browser <id>]
//!   omarchy-chromium profiles <id>
//!   omarchy-chromium args <id> [url] [--private] [--profile <name>]
//!   omarchy-chromium flag <flag>     (add extra flag)

use omarchy_chromium::{Browser, OmarchyChromium, Profile};
use std::process::exit;

fn seed() -> OmarchyChromium {
    let mut m = OmarchyChromium::new();
    let mut detected = OmarchyChromium::detect();
    if detected.is_empty() {
        // demo fallback
        detected.push(Browser {
            id: "chromium".into(), name: "Chromium".into(),
            kind: omarchy_chromium::BrowserKind::Chromium,
            binary: "/usr/bin/chromium".into(),
            profiles: vec![Profile { name: "Default".into(), path: "~/.config/chromium/Default".into(), guest: false }],
            default_profile: "Default".into(),
        });
    }
    for b in detected { let _ = m.register(b); }
    m
}

fn usage() -> ! {
    eprintln!("usage: omarchy-chromium <detect|list|default|open|profiles|args|flag> [args]");
    exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() { usage(); }
    let state = OmarchyChromium::state_path();
    let mut m = seed();
    m.load_state(&state);
    let cmd = args[0].as_str();
    let rest = &args[1..];

    let result: Result<(), String> = (|| -> Result<(), String> { match cmd {
        "detect" => {
            let found = OmarchyChromium::detect();
            for b in &found { println!("{} ({}) -> {}", b.name, b.kind.label(), b.binary.display()); }
            println!("{} detected", found.len());
            Ok(())
        }
        "list" => {
            for b in m.browsers() {
                println!("{:<16} {:<10} {} ({} profiles)", b.name, b.kind.label(), b.binary.display(), b.profiles.len());
            }
            eprintln!("{}", m.status_line());
            Ok(())
        }
        "default" => {
            match rest.first() {
                Some(id) => { m.set_default(id).map_err(|e| e.to_string())?; println!("default={id}"); }
                None => println!("default={}", m.default().map(|b| b.id.as_str()).unwrap_or("none")),
            }
            Ok(())
        }
        "open" => {
            let url = rest.first().map(|s| s.as_str()).unwrap_or_else(|| usage());
            let priv_ = rest.iter().any(|s| s == "--private" || s == "-p");
            let browser = rest.iter().position(|s| s == "--browser" || s == "-b")
                .and_then(|i| rest.get(i + 1)).map(|s| s.as_str());
            let argv = match browser {
                Some(id) => m.launch_args(id, Some(url), None, priv_),
                None => m.open_url(url, priv_),
            }.map_err(|e| e.to_string())?;
            println!("{}", argv.join(" "));
            Ok(())
        }
        "profiles" => {
            let id = rest.first().map(|s| s.as_str()).unwrap_or_else(|| usage());
            match m.get(id) {
                Some(b) => { for p in &b.profiles { println!("{}{}", p.name, if p.guest { " (guest)" } else { "" }); } Ok(()) }
                None => Err(format!("no such browser {id}")),
            }
        }
        "args" => {
            let id = rest.first().map(|s| s.as_str()).unwrap_or_else(|| usage());
            let url = rest.get(1).filter(|s| !s.starts_with('-')).map(|s| s.as_str());
            let priv_ = rest.iter().any(|s| s == "--private" || s == "-p");
            let prof = rest.iter().position(|s| s == "--profile" || s == "-P")
                .and_then(|i| rest.get(i + 1)).map(|s| s.as_str());
            let argv = m.launch_args(id, url, prof, priv_).map_err(|e| e.to_string())?;
            println!("{}", argv.join(" "));
            Ok(())
        }
        "flag" => {
            let f = rest.first().map(|s| s.as_str()).unwrap_or_else(|| usage());
            m.add_flag(f); println!("added flag {f}"); Ok(())
        }
        _ => usage(),
    } })();

    match result {
        Ok(()) => {
            if let Err(e) = m.save_state(&state) { eprintln!("omarchy-chromium: warn: save: {e}"); }
        }
        Err(e) => { eprintln!("omarchy-chromium: {e}"); exit(1); }
    }
}
