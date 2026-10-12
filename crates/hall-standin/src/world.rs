//! The world file: a declarative description of the test setup's reeves and
//! errands, kept at `<test reeve home>/standin/world.toml`. Editing it is how a
//! hand state, a question or a flag is set (plan step 1). See docs/listing-draft.md.

use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, Item, Table, value};

use crate::home::is_live_herdr_session;

/// Where the world file sits inside a test reeve home.
pub fn world_path(home: &Path) -> PathBuf {
    home.join("standin").join("world.toml")
}

/// Where an errand stands, as the reeve reconciles it from the hand's status file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HandState {
    #[default]
    Working,
    NeedsDecision,
    Blocked,
    Done,
    Failed,
}

impl HandState {
    pub const ALL: [HandState; 5] = [
        HandState::Working,
        HandState::NeedsDecision,
        HandState::Blocked,
        HandState::Done,
        HandState::Failed,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            HandState::Working => "working",
            HandState::NeedsDecision => "needs-decision",
            HandState::Blocked => "blocked",
            HandState::Done => "done",
            HandState::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<HandState> {
        HandState::ALL.into_iter().find(|h| h.as_str() == s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reeve {
    pub name: String,
    pub herdr_session: String,
    #[serde(default)]
    pub pane_id: Option<String>,
    #[serde(default = "yes")]
    pub live: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Errand {
    pub id: String,
    pub office: String,
    pub owner: String,
    #[serde(default)]
    pub state: HandState,
    #[serde(default)]
    pub question: Option<String>,
    #[serde(default)]
    pub waiting: bool,
    #[serde(default)]
    pub idle: bool,
    #[serde(default)]
    pub wedged: bool,
    #[serde(default = "yes")]
    pub in_flight: bool,
    pub herdr_session: String,
    #[serde(default)]
    pub pane_id: Option<String>,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct World {
    #[serde(default)]
    pub reeve: Vec<Reeve>,
    #[serde(default)]
    pub errand: Vec<Errand>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct WorldError(pub String);

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for WorldError {}

fn err<T>(msg: impl Into<String>) -> Result<T, WorldError> {
    Err(WorldError(msg.into()))
}

impl World {
    /// Parses and checks a world file's text.
    pub fn parse(text: &str) -> Result<World, WorldError> {
        let world: World = toml_edit::de::from_str(text).map_err(|e| WorldError(e.to_string()))?;
        world.check()?;
        Ok(world)
    }

    pub fn load(home: &Path) -> Result<World, WorldError> {
        let path = world_path(home);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| WorldError(format!("cannot read world file {}: {e}", path.display())))?;
        World::parse(&text).map_err(|e| WorldError(format!("{}: {e}", path.display())))
    }

    fn check(&self) -> Result<(), WorldError> {
        let mut names = HashSet::new();
        for r in &self.reeve {
            if r.name.is_empty() {
                return err("a reeve has an empty name");
            }
            if !names.insert(r.name.as_str()) {
                return err(format!("reeve {} is named twice", r.name));
            }
            check_location(&format!("reeve {}", r.name), &r.herdr_session)?;
        }
        let mut ids = HashSet::new();
        for e in &self.errand {
            if e.id.is_empty() {
                return err("an errand has an empty id");
            }
            if !ids.insert(e.id.as_str()) {
                return err(format!("errand {} is listed twice", e.id));
            }
            if names.contains(e.id.as_str()) {
                return err(format!("errand {} has the same name as a reeve", e.id));
            }
            if !names.contains(e.owner.as_str()) {
                return err(format!(
                    "errand {} names owner reeve {}, which is not in the world file",
                    e.id, e.owner
                ));
            }
            check_location(&format!("errand {}", e.id), &e.herdr_session)?;
        }
        Ok(())
    }
}

fn check_location(who: &str, herdr_session: &str) -> Result<(), WorldError> {
    if herdr_session.is_empty() {
        return err(format!("{who} has an empty herdr_session"));
    }
    if is_live_herdr_session(herdr_session) {
        return err(format!(
            "refusing {who}: herdr session {herdr_session} belongs to the live setup"
        ));
    }
    Ok(())
}

/// The keys `set` may change on a reeve.
pub const REEVE_KEYS: &[&str] = &["live"];
/// The keys `set` may change on an errand.
pub const ERRAND_KEYS: &[&str] = &[
    "state",
    "question",
    "waiting",
    "idle",
    "wedged",
    "in_flight",
];

/// Edits the world file's text in place, keeping its comments and layout, and
/// returns the new text only if it still parses and checks as a world.
pub struct WorldEdit {
    doc: DocumentMut,
}

impl WorldEdit {
    pub fn parse(text: &str) -> Result<WorldEdit, WorldError> {
        World::parse(text)?;
        let doc = text
            .parse::<DocumentMut>()
            .map_err(|e| WorldError(e.to_string()))?;
        Ok(WorldEdit { doc })
    }

    /// Sets `key=raw` on the reeve or errand called `target`.
    pub fn set(&mut self, target: &str, key: &str, raw: &str) -> Result<(), WorldError> {
        let (kind, table) = self.find(target)?;
        let keys = if kind == "reeve" {
            REEVE_KEYS
        } else {
            ERRAND_KEYS
        };
        if !keys.contains(&key) {
            return err(format!(
                "{kind} {target} has no settable key {key} (settable: {})",
                keys.join(", ")
            ));
        }
        match key {
            "state" => {
                let state = HandState::parse(raw).ok_or_else(|| {
                    WorldError(format!(
                        "unknown hand state {raw} (one of: {})",
                        HandState::ALL.map(HandState::as_str).join(", ")
                    ))
                })?;
                table.insert(key, value(state.as_str()));
            }
            "question" if raw.is_empty() => {
                table.remove(key);
            }
            "question" => {
                table.insert(key, value(raw));
            }
            _ => {
                let flag = match raw {
                    "true" => true,
                    "false" => false,
                    _ => return err(format!("{key} takes true or false, not {raw}")),
                };
                table.insert(key, value(flag));
            }
        }
        Ok(())
    }

    /// Records where `target`'s session sits, or forgets the pane when `pane_id` is None.
    pub fn place(
        &mut self,
        target: &str,
        herdr_session: &str,
        pane_id: Option<&str>,
    ) -> Result<(), WorldError> {
        check_location(target, herdr_session)?;
        let (_, table) = self.find(target)?;
        table.insert("herdr_session", value(herdr_session));
        match pane_id {
            Some(p) if !p.is_empty() => {
                table.insert("pane_id", value(p));
            }
            _ => {
                table.remove("pane_id");
            }
        }
        Ok(())
    }

    /// The edited text, checked as a world before it is handed back.
    pub fn finish(self) -> Result<String, WorldError> {
        let text = self.doc.to_string();
        World::parse(&text)?;
        Ok(text)
    }

    fn find(&mut self, target: &str) -> Result<(&'static str, &mut Table), WorldError> {
        let found = [("reeve", "name"), ("errand", "id")]
            .into_iter()
            .find_map(|(kind, field)| {
                let Some(Item::ArrayOfTables(tables)) = self.doc.get(kind) else {
                    return None;
                };
                tables
                    .iter()
                    .position(|t| t.get(field).and_then(|v| v.as_str()) == Some(target))
                    .map(|i| (kind, i))
            });
        let Some((kind, i)) = found else {
            return err(format!(
                "no reeve or errand called {target} in the world file"
            ));
        };
        match self.doc.get_mut(kind) {
            Some(Item::ArrayOfTables(tables)) => {
                Ok((kind, tables.get_mut(i).expect("found above")))
            }
            _ => unreachable!("found above"),
        }
    }
}

/// Where a reeve's or errand's session sits, by name.
pub fn locate<'a>(world: &'a World, target: &str) -> Option<(&'a str, Option<&'a str>)> {
    world
        .reeve
        .iter()
        .find(|r| r.name == target)
        .map(|r| (r.herdr_session.as_str(), r.pane_id.as_deref()))
        .or_else(|| {
            world
                .errand
                .iter()
                .find(|e| e.id == target)
                .map(|e| (e.herdr_session.as_str(), e.pane_id.as_deref()))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORLD: &str = r#"
# a comment that survives edits
[[reeve]]
name = "ansel"
herdr_session = "hall-dev"

[[errand]]
id = "ansel-fix"
office = "artificer"
owner = "ansel"
herdr_session = "hall-dev-2"
"#;

    #[test]
    fn defaults() {
        let w = World::parse(WORLD).unwrap();
        assert!(w.reeve[0].live);
        assert_eq!(w.reeve[0].pane_id, None);
        let e = &w.errand[0];
        assert_eq!(e.state, HandState::Working);
        assert_eq!(e.question, None);
        assert!(!e.waiting && !e.idle && !e.wedged);
        assert!(e.in_flight);
    }

    #[test]
    fn refuses_default_session() {
        let text = WORLD.replace("\"hall-dev-2\"", "\"default\"");
        let e = World::parse(&text).unwrap_err();
        assert!(e.0.contains("live setup"), "{e}");
    }

    #[test]
    fn refuses_unknown_owner_and_duplicates() {
        assert!(World::parse(&WORLD.replace("owner = \"ansel\"", "owner = \"nobody\"")).is_err());
        let twice = format!("{WORLD}\n[[reeve]]\nname = \"ansel\"\nherdr_session = \"hall-dev\"\n");
        assert!(World::parse(&twice).is_err());
    }

    #[test]
    fn refuses_unknown_keys_and_states() {
        assert!(World::parse(&WORLD.replace("office =", "offize =")).is_err());
        let bad = WORLD.replace(
            "office = \"artificer\"",
            "office = \"artificer\"\nstate = \"stuck\"",
        );
        assert!(World::parse(&bad).is_err());
    }

    #[test]
    fn set_keeps_comments_and_checks_values() {
        let mut ed = WorldEdit::parse(WORLD).unwrap();
        ed.set("ansel-fix", "state", "needs-decision").unwrap();
        ed.set("ansel-fix", "question", "Which base?").unwrap();
        ed.set("ansel-fix", "wedged", "true").unwrap();
        ed.set("ansel", "live", "false").unwrap();
        assert!(ed.set("ansel-fix", "state", "stuck").is_err());
        assert!(ed.set("ansel-fix", "idle", "yes").is_err());
        assert!(ed.set("ansel", "state", "blocked").is_err());
        assert!(ed.set("nobody", "state", "blocked").is_err());
        let text = ed.finish().unwrap();
        assert!(text.contains("# a comment that survives edits"));
        let w = World::parse(&text).unwrap();
        assert_eq!(w.errand[0].state, HandState::NeedsDecision);
        assert_eq!(w.errand[0].question.as_deref(), Some("Which base?"));
        assert!(w.errand[0].wedged);
        assert!(!w.reeve[0].live);

        let mut ed = WorldEdit::parse(&text).unwrap();
        ed.set("ansel-fix", "question", "").unwrap();
        assert_eq!(
            World::parse(&ed.finish().unwrap()).unwrap().errand[0].question,
            None
        );
    }

    #[test]
    fn place_records_and_refuses_default() {
        let mut ed = WorldEdit::parse(WORLD).unwrap();
        ed.place("ansel", "hall-dev", Some("w1:p1")).unwrap();
        assert!(ed.place("ansel-fix", "default", Some("w1:p2")).is_err());
        let w = World::parse(&ed.finish().unwrap()).unwrap();
        assert_eq!(locate(&w, "ansel"), Some(("hall-dev", Some("w1:p1"))));
        assert_eq!(locate(&w, "ansel-fix"), Some(("hall-dev-2", None)));
        assert_eq!(locate(&w, "nobody"), None);
    }
}
