//! Fork: usage-aware model choice (docs/agents.md item 7). A role may have an alternate model in settings
//! (`md["<role>.alt"]`). When the first choice's plan is past the role's threshold and the alternate's isn't, the
//! alternate runs and the reason is logged. No alternate, no change.
use crate::app::{App, Ax};
use serde_json::{json, Value};

/// Weekly-use threshold (%) per role: cheap triage moves first, the Fixer last.
pub fn tier(w: &str) -> f64 {
    match w {
        "scout" | "guard" | "vet" | "watcher" | "hunter" => 70.0,
        "repro" | "repro2" | "reviewer" | "submit" => 85.0,
        _ => 95.0,
    }
}

/// (weekly used %, 5-hour used %) for a provider from a stored usage reading.
fn used(u: &Value, id: &str) -> Option<(f64, f64)> {
    let p = u["providers"].as_array()?.iter().find(|p| p["id"] == id && p["available"] == true)?;
    let w = |k: &str| p["windows"].as_array().into_iter().flatten().find(|x| x["kind"] == k).and_then(|x| x["remaining"].as_f64()).map(|r| 100.0 - r);
    Some((w("weekly").unwrap_or(0.0), w("five_hour").unwrap_or(0.0)))
}

/// Why this provider shouldn't take this role now, if it shouldn't. Unknown usage counts as fine.
pub fn over(u: &Value, md: &str, th: f64) -> Option<String> {
    let id = crate::prov::parse(md).0;
    let (wk, fh) = used(u, id)?;
    let n = if id == "cc" { "Claude" } else { "Codex" };
    if fh >= 90.0 {
        Some(format!("{n} 5-hour window at {fh:.0} %"))
    } else if wk >= th {
        Some(format!("{n} weekly at {wk:.0} % (limit for this role {th:.0} %)"))
    } else {
        None
    }
}

/// The model to run and, when it isn't the first choice, why.
pub fn choose(w: &str, first: &str, alt: &str, u: &Value) -> (String, Option<String>) {
    if alt.is_empty() || alt == first {
        return (first.into(), None);
    }
    let th = tier(w);
    match over(u, first, th) {
        Some(why) if over(u, alt, th).is_none() => (alt.into(), Some(why)),
        _ => (first.into(), None),
    }
}

/// Called from `App::md` with the configured model.
pub fn adjust(a: &App, w: &str, first: String) -> String {
    let alt = a.set()["md"][format!("{w}.alt")].as_str().unwrap_or("").trim().to_string();
    if alt.is_empty() {
        return first;
    }
    let (m, why) = choose(w, &first, &alt, &a.db.kg("usage:last"));
    let k = format!("ua:{w}");
    if a.db.kg(&k).as_str() != Some(m.as_str()) {
        a.db.ks(&k, &json!(m));
        a.say(w, &match why {
            Some(r) => format!("using {m} instead of {first}: {r}"),
            None => format!("back on {m}"),
        }, None);
    }
    m
}

/// Keep a usage reading for `adjust` (every 10 min; usage.rs caches per provider, so this adds no extra calls).
pub async fn watch(a: Ax) {
    loop {
        let has_alt = a.set()["md"].as_object().is_some_and(|m| m.keys().any(|k| k.ends_with(".alt")));
        if has_alt {
            let mut v = crate::usage::read(&a).await;
            v["t"] = json!(crate::db::now());
            a.db.ks("usage:last", &v);
        }
        crate::app::sl(600_000).await;
    }
}

#[cfg(test)]
mod t {
    use super::*;

    fn u(cc_wk: f64, cc_5h: f64, cx_wk: f64) -> Value {
        json!({"providers": [
            {"id": "cc", "available": true, "windows": [{"kind": "weekly", "remaining": 100.0 - cc_wk}, {"kind": "five_hour", "remaining": 100.0 - cc_5h}]},
            {"id": "codex", "available": true, "windows": [{"kind": "weekly", "remaining": 100.0 - cx_wk}]}
        ]})
    }

    #[test]
    fn picks() {
        assert_eq!(choose("vet", "claude-haiku-5-5", "codex:", &u(60.0, 10.0, 5.0)).0, "claude-haiku-5-5");
        let (m, why) = choose("vet", "claude-haiku-5-5", "codex:", &u(72.0, 10.0, 5.0));
        assert_eq!(m, "codex:");
        assert!(why.unwrap().contains("weekly at 72"));
        assert_eq!(choose("fixer", "claude-sonnet-5-5", "codex:", &u(72.0, 10.0, 5.0)).0, "claude-sonnet-5-5");
        assert_eq!(choose("fixer", "claude-sonnet-5-5", "codex:", &u(40.0, 95.0, 5.0)).0, "codex:");
        assert_eq!(choose("vet", "claude-haiku-5-5", "codex:", &u(80.0, 10.0, 90.0)).0, "claude-haiku-5-5");
        assert_eq!(choose("vet", "claude-haiku-5-5", "", &u(99.0, 99.0, 0.0)).0, "claude-haiku-5-5");
        assert_eq!(choose("vet", "claude-haiku-5-5", "codex:", &json!({})).0, "claude-haiku-5-5");
    }
}
