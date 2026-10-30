//! Fork: what the office knows about a repository (docs/agents.md item 8 R1), handed to workers as a skill.
//! Lives in `<data>/know/<owner__repo>/SKILL.md` plus any other `.md` next to it. No file, no change.
use crate::app::App;
use crate::pa;
use std::path::{Path, PathBuf};

pub const ROLES: &[&str] = &["vet", "repro", "fixer", "reviewer"];

pub fn dir(a: &App, repo: &str) -> PathBuf {
    a.dd.join("know").join(repo.replace('/', "__"))
}

/// The knowledge file for this call's job, when the role uses it and the repo has one.
pub fn file(a: &App, job: Option<i64>, w: &str) -> Option<PathBuf> {
    if !ROLES.contains(&w) {
        return None;
    }
    let r = a.db.one("select repo from jobs where id=?", pa![job?])?["repo"].as_str()?.to_string();
    let f = dir(a, &r).join("SKILL.md");
    f.is_file().then_some(f)
}

/// A throwaway working directory with the repo's notes as a project skill, for Claude Code workers.
pub fn stage(a: &App, f: &Path) -> Option<PathBuf> {
    let d = a.dd.join("run").join(format!("sk-{}", hex::encode(rand::random::<[u8; 8]>())));
    let s = d.join(".claude").join("skills").join("repo");
    std::fs::create_dir_all(&s).ok()?;
    for e in std::fs::read_dir(f.parent()?).ok()?.flatten() {
        if e.path().extension().is_some_and(|x| x == "md") {
            std::fs::copy(e.path(), s.join(e.file_name())).ok()?;
        }
    }
    Some(d)
}

/// For workers without native skills (Codex, API models): the notes' body, capped, as a trusted prompt block.
pub fn inline(f: &Path, w: &str) -> String {
    let s = std::fs::read_to_string(f).unwrap_or_default();
    format!("\n\nWhat the office knows about this repository (trusted, kept by the owner):\n{}{}", crate::sh::head(body(&s).trim(), 6000), check(w))
}

/// For Claude Code workers: the skill is there, ask for it explicitly so it isn't skipped.
pub fn nudge(w: &str) -> String {
    format!("\n\nThis repository has office notes in the `repo` skill (trusted, kept by the owner). Load it before you start.{}", check(w))
}

fn check(w: &str) -> &'static str {
    if w == "reviewer" {
        "\nCheck the change against the rules in these notes and the repository's own agent instructions they point to; list every violation as an issue."
    } else {
        ""
    }
}

fn body(s: &str) -> &str {
    s.strip_prefix("---").and_then(|r| r.split_once("\n---").map(|x| x.1)).unwrap_or(s)
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn front_matter() {
        assert_eq!(body("---\nname: repo\ndescription: x\n---\n# Brief\nok").trim(), "# Brief\nok");
        assert_eq!(body("# no front matter"), "# no front matter");
    }
}
