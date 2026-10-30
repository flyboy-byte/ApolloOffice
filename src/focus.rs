//! Fork: point the office at specific jobs ("this one next", "only this one tonight") and a compact state for
//! scripts. Used by the `/v1/focus` and `/v1/state` token API routes and `pipe::pick`. docs/manager.md.
use crate::app::App;
use crate::pa;
use serde_json::{json, Value};

/// Focused job ids that are still open, and whether only they may run. Finished jobs fall out; an empty focus clears.
pub fn live(a: &App) -> (Vec<i64>, bool) {
    let f = a.db.kg("focus");
    let ids: Vec<i64> = f["ids"].as_array().into_iter().flatten().filter_map(Value::as_i64).collect();
    let open: Vec<i64> = ids.iter().copied().filter(|id| a.db.jg(*id).is_some_and(|j| !["done", "drop"].contains(&j.st.as_str()))).collect();
    if open.is_empty() {
        if !ids.is_empty() {
            a.db.kd("focus");
            a.say("scout", "focus finished, back to normal work", None);
        }
        return (vec![], false);
    }
    let only = f["only"].as_bool().unwrap_or(false);
    if open.len() != ids.len() {
        a.db.ks("focus", &json!({"ids": open, "only": only}));
    }
    (open, only)
}

pub fn add(a: &App, id: i64, only: bool) {
    let (mut ids, o) = live(a);
    if !ids.contains(&id) {
        ids.push(id);
    }
    a.db.ks("focus", &json!({"ids": ids, "only": only || o}));
    a.say("scout", &format!("focus on job {id}{}", if only || o { ", nothing else starts" } else { "" }), Some(id));
}

pub fn clear(a: &App) {
    a.db.kd("focus");
    a.say("scout", "focus cleared", None);
}

/// Focused jobs first (stable), and with `only` nothing else except jobs already at submit.
pub fn order(c: &mut Vec<Value>, ids: &[i64], only: bool) {
    if ids.is_empty() {
        return;
    }
    let f = |j: &Value| ids.contains(&j["id"].as_i64().unwrap_or(0));
    c.sort_by_key(|j| !f(j));
    if only {
        c.retain(|j| f(j) || j["st"] == "submit");
    }
}

/// Everything a script needs to see what the office is doing, without the browser.
pub fn state(a: &App) -> Value {
    let (ids, only) = live(a);
    let lv = a.lv.lock().unwrap();
    let jobs: Vec<Value> = a
        .db
        .all("select id, repo, num, kind, title, st, why, t0, t1, prio from jobs where st not in ('done','drop') order by id", pa![])
        .into_iter()
        .map(|mut j| {
            let id = j["id"].as_i64().unwrap_or(0);
            j["ci"] = json!(lv.cil.get(&id));
            j["focus"] = json!(ids.contains(&id));
            j
        })
        .collect();
    let workers: Value = lv.w.iter().map(|(k, w)| (k.clone(), json!({"st": w.st, "say": w.say, "job": w.job, "md": w.md, "hold": w.hold, "t": w.t}))).collect::<serde_json::Map<_, _>>().into();
    let machines: Vec<Value> = a
        .db
        .all("select id, dk, ln, ci, on_ from hs order by id", pa![])
        .into_iter()
        .map(|mut h| {
            let id = h["id"].as_str().unwrap_or("").to_string();
            h["cpu"] = a.db.kg(&format!("cpu:{id}"));
            h["mem_mb"] = a.db.kg(&format!("mem:{id}"));
            h["scope"] = a.db.kg(&format!("scope:{id}"));
            h["hw"] = lv.hst.get(&id).cloned().unwrap_or(Value::Null);
            h
        })
        .collect();
    json!({"office": {"off": lv.o.off, "pause": lv.o.pause}, "focus": {"ids": ids, "only": only}, "jobs": jobs, "workers": workers, "machines": machines, "recent": a.db.all("select id, repo, num, st, why, t1 from jobs where st in ('done','drop') order by t1 desc limit 10", pa![])})
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn ordering() {
        let mut c = vec![json!({"id": 1, "st": "ready"}), json!({"id": 2, "st": "submit"}), json!({"id": 3, "st": "ready"})];
        order(&mut c, &[3], false);
        assert_eq!(c.iter().map(|j| j["id"].as_i64().unwrap()).collect::<Vec<_>>(), vec![3, 1, 2]);
        order(&mut c, &[3], true);
        assert_eq!(c.iter().map(|j| j["id"].as_i64().unwrap()).collect::<Vec<_>>(), vec![3, 2]);
        let mut d = vec![json!({"id": 1})];
        order(&mut d, &[], true);
        assert_eq!(d.len(), 1);
    }
}
