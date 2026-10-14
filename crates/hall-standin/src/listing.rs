//! The listing, version 1: the format reeve's own `bin/reeve-listing` prints
//! (decision 68), here from a world file so hall's tests and the test setup can
//! set any state on demand. Shape: docs/listing.md.

use serde::Serialize;

use crate::world::{HandState, World};

/// Where a reeve or a hand sits: the backend, the session there and the pane in
/// it. The stand-in names only hall sessions, so `backend` is always `hall`; the
/// world file keeps its key `herdr_session` for the session name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionLocation {
    pub backend: &'static str,
    pub name: String,
    /// None until the test setup has recorded the pane.
    pub pane_id: Option<String>,
}

impl SessionLocation {
    fn hall(name: &str, pane_id: &Option<String>) -> SessionLocation {
        SessionLocation {
            backend: "hall",
            name: name.to_owned(),
            pane_id: pane_id.clone(),
        }
    }
}

/// The listing format version, which hall requires.
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReeveEntry {
    pub name: String,
    pub session: SessionLocation,
    pub live: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrandEntry {
    pub id: String,
    pub office: String,
    pub owner_reeve: String,
    pub hand_state: HandState,
    pub question: Option<String>,
    pub waiting: bool,
    pub idle: bool,
    pub wedged: bool,
    pub session: SessionLocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Listing {
    /// Always `VERSION`.
    pub version: u32,
    pub reeves: Vec<ReeveEntry>,
    /// Errands in flight only (decision 16).
    pub errands: Vec<ErrandEntry>,
}

impl Listing {
    pub fn from_world(world: &World) -> Listing {
        Listing {
            version: VERSION,
            reeves: world
                .reeve
                .iter()
                .map(|r| ReeveEntry {
                    name: r.name.clone(),
                    session: SessionLocation::hall(&r.herdr_session, &r.pane_id),
                    live: r.live,
                })
                .collect(),
            errands: world
                .errand
                .iter()
                .filter(|e| e.in_flight)
                .map(|e| ErrandEntry {
                    id: e.id.clone(),
                    office: e.office.clone(),
                    owner_reeve: e.owner.clone(),
                    hand_state: e.state,
                    question: e.question.clone().filter(|q| !q.is_empty()),
                    waiting: e.waiting,
                    idle: e.idle,
                    wedged: e.wedged,
                    session: SessionLocation::hall(&e.herdr_session, &e.pane_id),
                })
                .collect(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a listing always serialises")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omits_errands_not_in_flight_and_empty_questions() {
        let w = World::parse(
            r#"
[[reeve]]
name = "ansel"
herdr_session = "hall-dev"
pane_id = "w1:p1"

[[errand]]
id = "kept"
office = "artificer"
owner = "ansel"
question = ""
herdr_session = "hall-dev"

[[errand]]
id = "gone"
office = "artificer"
owner = "ansel"
state = "done"
in_flight = false
herdr_session = "hall-dev"
"#,
        )
        .unwrap();
        let l = Listing::from_world(&w);
        assert_eq!(l.version, 1);
        assert_eq!(l.errands.len(), 1);
        assert_eq!(l.errands[0].id, "kept");
        assert_eq!(l.errands[0].question, None);
        let v: serde_json::Value = serde_json::from_str(&l.to_json()).unwrap();
        assert_eq!(v["version"], 1);
        assert_eq!(v.get("draft"), None);
        assert_eq!(v["reeves"][0]["session"]["backend"], "hall");
        assert_eq!(v["reeves"][0]["session"]["name"], "hall-dev");
        assert_eq!(v["reeves"][0]["session"]["pane_id"], "w1:p1");
        assert_eq!(v["errands"][0]["hand_state"], "working");
        assert_eq!(
            v["errands"][0]["session"]["pane_id"],
            serde_json::Value::Null
        );
    }
}
