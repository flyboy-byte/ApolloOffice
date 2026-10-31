//! Fork: rent a Linode build box for N minutes, with a layered dead-man switch (docs/cloud-hosts.md).
//! Off unless settings `cloud.on` is true and LINODE_TOKEN is set in the office's environment. Spend guards first.
//! L1 here (drain at TTL, then delete), L2 here (delete any expired `ao-exp-*` box the API still lists),
//! L3 `cloud/reaper.py` on the VPS, L4 the box powers itself off (stops work, not billing).
use crate::app::{App, Ax};
use crate::db::now;
use crate::pa;
use crate::sh::{hraw, Ho};
use serde_json::{json, Value};

const API: &str = "https://api.linode.com/v4";
const RAW: &str = "https://raw.githubusercontent.com/flyboy-byte/ApolloOffice/main/cloud";
/// Host keys of rented boxes; ssh reads it next to the user's own known_hosts (`sh::sshv`).
pub const KH: &str = "~/.ssh/ao_cloud_known_hosts";
pub const PRE: &str = "lin-";

pub struct Lim {
    pub on: bool,
    pub max: i64,
    pub max_min: i64,
    pub day: i64,
    pub types: Vec<String>,
}

pub fn lim(a: &App) -> Lim {
    let c = &a.set()["cloud"];
    Lim {
        on: c["on"].as_bool() == Some(true),
        max: c["max"].as_i64().unwrap_or(1),
        max_min: c["max_min"].as_i64().unwrap_or(60),
        day: c["day"].as_i64().unwrap_or(4),
        types: c["types"].as_array().map(|v| v.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_else(|| vec!["g6-standard-6".into()]),
    }
}

/// Spend guards, checked before every create.
pub fn guard(l: &Lim, open: i64, today: i64, minutes: i64, ty: &str) -> Result<(), String> {
    if !l.on {
        return Err("cloud boxes are off (settings cloud.on)".into());
    }
    if !l.types.iter().any(|t| t == ty) {
        return Err(format!("type {ty} is not in the allowed list {:?}", l.types));
    }
    if minutes < 10 || minutes > l.max_min {
        return Err(format!("minutes must be 10..={}", l.max_min));
    }
    if open >= l.max {
        return Err(format!("{open} box(es) already running (max {})", l.max));
    }
    if today >= l.day {
        return Err(format!("{today} box(es) rented today (max {})", l.day));
    }
    Ok(())
}

/// cloud-init for the box: our host key (pinned before boot), a hard power-off (L4), Docker, then cloud/up.sh.
pub fn user_data(hk_priv: &str, hk_pub: &str, hard_min: i64, repo: &str) -> String {
    let pk: String = hk_priv.lines().map(|l| format!("    {l}\n")).collect();
    let repo: String = repo.chars().filter(|c| c.is_ascii_alphanumeric() || "/._-".contains(*c)).collect();
    format!(
        "#cloud-config\nssh_deletekeys: true\nssh_keys:\n  ed25519_private: |\n{pk}  ed25519_public: {}\nruncmd:\n  - [sh, -c, \"shutdown -h +{hard_min}\"]\n  - [sh, -c, \"apt-get update -q && DEBIAN_FRONTEND=noninteractive apt-get install -yq docker.io docker-compose-v2\"]\n  - [sh, -c, \"curl -fsSL {RAW}/up.sh | REPO={repo} sh > /var/log/ao-up.log 2>&1\"]\n",
        hk_pub.trim()
    )
}

/// `ao-exp-<unix seconds>` from a box's tags.
pub fn exp(tags: &Value) -> Option<i64> {
    tags.as_array()?.iter().filter_map(|t| t.as_str()?.strip_prefix("ao-exp-")?.parse().ok()).next()
}

fn tok() -> Result<String, String> {
    std::env::var("LINODE_TOKEN").ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).ok_or_else(|| "LINODE_TOKEN is not set in the office's environment".into())
}

async fn api(a: &App, m: reqwest::Method, p: &str, b: Option<Value>) -> Result<Value, String> {
    let mut r = a.hc.request(m, format!("{API}{p}")).bearer_auth(tok()?);
    if p.starts_with("/linode/instances?") {
        r = r.header("X-Filter", json!({"tags": "apollo-office"}).to_string());
    }
    if let Some(b) = b {
        r = r.json(&b);
    }
    let r = r.send().await.map_err(|e| e.to_string())?;
    let st = r.status();
    let v: Value = r.json().await.unwrap_or(Value::Null);
    if !st.is_success() {
        return Err(format!("Linode {st}: {}", v["errors"].to_string().chars().take(300).collect::<String>()));
    }
    Ok(v)
}

fn kh_path() -> std::path::PathBuf {
    dirs::home_dir().unwrap_or_default().join(".ssh").join("ao_cloud_known_hosts")
}

fn kh_set(ip: &str, line: Option<&str>) {
    let p = kh_path();
    let mut ls: Vec<String> = std::fs::read_to_string(&p).unwrap_or_default().lines().filter(|l| !l.starts_with(&format!("{ip} "))).map(String::from).collect();
    if let Some(l) = line {
        ls.push(format!("{ip} {}", l.trim()));
    }
    let _ = std::fs::write(&p, ls.join("\n") + "\n");
}

fn boxes(a: &App) -> Vec<Value> {
    a.db.all("select v from kv where k like 'cloud:box:%'", pa![]).into_iter().filter_map(|r| r["v"].as_str().and_then(|s| serde_json::from_str(s).ok()).or(Some(r["v"].clone()))).filter(|v: &Value| v.is_object()).collect()
}

fn ho(id: &str, ip: &str) -> Ho {
    Ho { id: id.into(), ssh: format!("root@{ip}"), dk: true, ln: 1, ci: 1, gb: 100, on: true, no: vec![], cpu: 0 }
}

/// Create a box. Returns at once; a task waits for cloud-init and then registers it as a machine.
pub async fn rent(a: &Ax, minutes: i64, ty: &str, region: &str, repo: &str) -> Result<Value, String> {
    let day = chrono::Local::now().format("%Y-%m-%d").to_string();
    let today = a.db.kg(&format!("cloud:day:{day}")).as_i64().unwrap_or(0);
    guard(&lim(a), boxes(a).len() as i64, today, minutes, ty)?;
    let pk = std::fs::read_to_string(dirs::home_dir().unwrap_or_default().join(".ssh/id_ed25519.pub")).map_err(|_| "no ~/.ssh/id_ed25519.pub to log in with".to_string())?;
    let kf = a.dd.join("run").join(format!("hk-{}", hex::encode(rand::random::<[u8; 6]>())));
    let _ = std::fs::create_dir_all(kf.parent().unwrap());
    let r = crate::sh::run(&crate::sh::tool("ssh-keygen"), &["-q".into(), "-t".into(), "ed25519".into(), "-N".into(), "".into(), "-C".into(), "ao-box".into(), "-f".into(), kf.display().to_string()], crate::sh::Rq { t: 30, ..Default::default() }).await;
    let (hpriv, hpub) = (std::fs::read_to_string(&kf).unwrap_or_default(), std::fs::read_to_string(kf.with_extension("pub")).unwrap_or_default());
    let _ = std::fs::remove_file(&kf);
    let _ = std::fs::remove_file(kf.with_extension("pub"));
    if r.c != 0 || hpriv.is_empty() {
        return Err(format!("ssh-keygen failed: {}", r.e));
    }
    let e = now() / 1000 + minutes * 60;
    use base64::Engine;
    let ud = base64::engine::general_purpose::STANDARD.encode(user_data(&hpriv, &hpub, minutes + 15, repo));
    let label = format!("ao-{}", hex::encode(rand::random::<[u8; 3]>()));
    let v = api(a, reqwest::Method::POST, "/linode/instances", Some(json!({
        "label": label, "region": region, "type": ty, "image": "linode/ubuntu24.04", "booted": true,
        "root_pass": format!("Ao!{}", hex::encode(rand::random::<[u8; 20]>())), "authorized_keys": [pk.trim()],
        "tags": ["apollo-office", format!("ao-exp-{e}")], "metadata": {"user_data": ud}
    }))).await?;
    let lid = v["id"].as_i64().ok_or("Linode returned no id")?;
    let ip = v["ipv4"][0].as_str().unwrap_or("").to_string();
    let hid = format!("{PRE}{lid}");
    // Host key pinned before first contact: hpub is "ssh-ed25519 AAAA… ao-box".
    kh_set(&ip, Some(hpub.split_whitespace().take(2).collect::<Vec<_>>().join(" ").as_str()));
    // Hourly price for the dashboard's cost estimate; Linode bills every started hour until the box is deleted.
    let px = match api(a, reqwest::Method::GET, &format!("/linode/types/{ty}"), None).await {
        Ok(t) => t["price"]["hourly"].as_f64(),
        Err(_) => None,
    };
    let b = json!({"lid": lid, "hid": hid, "ip": ip, "exp": e, "ty": ty, "region": region, "repo": repo, "t0": now(), "st": "boot", "px": px});
    a.db.ks(&format!("cloud:box:{lid}"), &b);
    a.db.ks(&format!("cloud:day:{day}"), &json!(today + 1));
    a.say("ci", &format!("[{hid}] rented {ty} in {region} for {minutes} min ({ip}), waiting for setup"), None);
    let a2 = a.clone();
    tokio::spawn(async move { up(&a2, lid, &hid, &ip).await });
    Ok(b)
}

async fn up(a: &Ax, lid: i64, hid: &str, ip: &str) {
    let h = ho(hid, ip);
    for _ in 0..40 {
        crate::app::sl(30_000).await;
        if a.db.kg(&format!("cloud:box:{lid}")).is_null() {
            return;
        }
        let r = hraw(&h, "cloud-init status --wait >/dev/null 2>&1; docker inspect -f '{{.State.Running}}' apollo-office 2>/dev/null", 900).await;
        if r.o.trim() == "true" {
            a.db.run("insert into hs(id,ssh,dk,ln,ci,gb,raw,on_,t0,no,cpu) values(?,?,1,1,1,100,0,1,?,'',0) on conflict(id) do update set on_=1", pa![hid.to_string(), h.ssh.clone(), now()]);
            let mut b = a.db.kg(&format!("cloud:box:{lid}"));
            if b["st"] == "boot" {
                b["st"] = json!("up");
            }
            a.db.ks(&format!("cloud:box:{lid}"), &b);
            a.say("ci", &format!("[{hid}] box ready, lanes start on it"), None);
            return;
        }
    }
    a.say("ci", &format!("[{hid}] box never became ready, deleting it"), None);
    kill(a, lid, true);
}

/// Ask for a box to go; `watch` deletes it. Wind down (`now` false): stop new work, delete once its job ends (at most 10 min). Stop now: delete at the next
/// watch tick (~30 s) even mid-job.
pub fn kill(a: &App, lid: i64, now_: bool) {
    let k = format!("cloud:box:{lid}");
    let mut b = a.db.kg(&k);
    if b.is_object() {
        b["exp"] = json!(now() / 1000 - if now_ { 601 } else { 0 });
        b["st"] = json!(if now_ { "stopping" } else { "winding down" });
        a.db.ks(&k, &b);
    }
}

async fn delete(a: &App, b: &Value) -> bool {
    let lid = b["lid"].as_i64().unwrap_or(0);
    match api(a, reqwest::Method::DELETE, &format!("/linode/instances/{lid}"), None).await {
        Ok(_) => true,
        Err(e) if e.contains("404") => true,
        Err(e) => {
            a.say("ci", &format!("[{PRE}{lid}] delete failed, will retry: {e}"), None);
            false
        }
    }
}

/// L1 + L2: drain boxes near their expiry, delete them once idle (or at the hard limit), and delete any expired box
/// the API still lists that this office doesn't track (crash, lost state).
pub async fn watch(a: Ax) {
    let mut n = 0u64;
    loop {
        crate::app::sl(30_000).await;
        n += 1;
        if tok().is_err() {
            continue;
        }
        let t = now() / 1000;
        for b in boxes(&a) {
            let (lid, hid, e) = (b["lid"].as_i64().unwrap_or(0), b["hid"].as_str().unwrap_or("").to_string(), b["exp"].as_i64().unwrap_or(0));
            if t < e - 300 {
                continue;
            }
            a.db.run("update hs set on_=0 where id=?", pa![hid.clone()]);
            let busy = a.db.n("select count(*) from jobs where json_extract(d,'$.h')=? and st in ('repro','fix','ci','review','gate','submit')", pa![hid.clone()]);
            if busy > 0 && t < e + 600 {
                continue;
            }
            let ap = a.db.n("select count(*) from jobs where json_extract(d,'$.h')=? and st='approve'", pa![hid.clone()]);
            if ap > 0 {
                a.say("ci", &format!("[{hid}] {ap} job(s) waiting for approval had their work on this box; it is lost with the box"), None);
            }
            if delete(&a, &b).await {
                a.db.run("delete from hs where id=?", pa![hid.clone()]);
                a.db.kd(&format!("cloud:box:{lid}"));
                kh_set(b["ip"].as_str().unwrap_or(""), None);
                a.say("ci", &format!("[{hid}] box deleted"), None);
            }
        }
        if n % 20 == 1 {
            if let Ok(v) = api(&a, reqwest::Method::GET, "/linode/instances?page_size=100", None).await {
                for i in v["data"].as_array().into_iter().flatten() {
                    if exp(&i["tags"]).is_some_and(|e| t > e + 900) {
                        let b = json!({"lid": i["id"], "ip": i["ipv4"][0]});
                        if delete(&a, &b).await {
                            a.say("ci", &format!("[{PRE}{}] expired box deleted (it was not tracked)", i["id"]), None);
                        }
                    }
                }
            }
        }
    }
}

pub fn list(a: &App) -> Value {
    let l = lim(a);
    let today = a.db.kg(&format!("cloud:day:{}", chrono::Local::now().format("%Y-%m-%d"))).as_i64().unwrap_or(0);
    json!({"on": l.on, "token": tok().is_ok(), "boxes": boxes(a), "max": l.max, "max_min": l.max_min, "day": l.day, "types": l.types, "today": today})
}

/// Settings page: `{"cloud": {"on", "max", "max_min", "day", "types"}}` merged into settings `cloud`.
pub fn sets(s: &mut Value, b: &Value) {
    let Some(c) = b["cloud"].as_object() else { return };
    if !s["cloud"].is_object() {
        s["cloud"] = json!({});
    }
    if let Some(x) = c.get("on").and_then(|x| x.as_bool()) {
        s["cloud"]["on"] = json!(x);
    }
    for (k, lo, hi) in [("max", 1, 3), ("max_min", 10, 240), ("day", 0, 24)] {
        if let Some(x) = c.get(k).and_then(|x| x.as_i64()) {
            s["cloud"][k] = json!(x.clamp(lo, hi));
        }
    }
    if let Some(x) = c.get("types").and_then(|x| x.as_str()) {
        let re = regex::Regex::new(r"^g\d-[a-z0-9-]{1,40}$").unwrap();
        let t: Vec<&str> = x.split(',').map(str::trim).filter(|t| re.is_match(t)).take(5).collect();
        if !t.is_empty() {
            s["cloud"]["types"] = json!(t);
        }
    }
}

#[cfg(test)]
mod t {
    use super::*;

    fn l() -> Lim {
        Lim { on: true, max: 1, max_min: 60, day: 4, types: vec!["g6-standard-6".into()] }
    }

    #[test]
    fn guards() {
        assert!(guard(&l(), 0, 0, 30, "g6-standard-6").is_ok());
        assert!(guard(&Lim { on: false, ..l() }, 0, 0, 30, "g6-standard-6").unwrap_err().contains("off"));
        assert!(guard(&l(), 0, 0, 30, "g7-premium-64").unwrap_err().contains("allowed"));
        assert!(guard(&l(), 0, 0, 90, "g6-standard-6").unwrap_err().contains("minutes"));
        assert!(guard(&l(), 1, 0, 30, "g6-standard-6").unwrap_err().contains("already running"));
        assert!(guard(&l(), 0, 4, 30, "g6-standard-6").unwrap_err().contains("today"));
    }

    #[test]
    fn cloud_init() {
        let u = user_data("-----BEGIN\nabc\n-----END\n", "ssh-ed25519 AAAA ao-box\n", 45, "tangency-labs/solidcraft; rm -rf /");
        assert!(u.starts_with("#cloud-config\n"));
        assert!(u.contains("  ed25519_private: |\n    -----BEGIN\n    abc\n    -----END\n  ed25519_public: ssh-ed25519 AAAA ao-box\n"));
        assert!(u.contains("shutdown -h +45"));
        assert!(u.contains("REPO=tangency-labs/solidcraftrm-rf/ sh"));
        assert_eq!(exp(&json!(["apollo-office", "ao-exp-1793400000"])), Some(1793400000));
        assert_eq!(exp(&json!(["other"])), None);
    }

    #[test]
    fn settings_page() {
        let mut s = json!({"ntfy": "x"});
        sets(&mut s, &json!({}));
        assert_eq!(s, json!({"ntfy": "x"}));
        sets(&mut s, &json!({"cloud": {"on": true, "max": 9, "max_min": 5, "day": 2, "types": "g6-standard-6, ; rm -rf, g6-dedicated-2"}}));
        assert_eq!(s["cloud"], json!({"on": true, "max": 3, "max_min": 10, "day": 2, "types": ["g6-standard-6", "g6-dedicated-2"]}));
        sets(&mut s, &json!({"cloud": {"on": false, "types": "nonsense"}}));
        assert_eq!(s["cloud"]["on"], json!(false));
        assert_eq!(s["cloud"]["types"], json!(["g6-standard-6", "g6-dedicated-2"]));
    }
}
