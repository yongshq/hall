//! The draft listing: the fields hall needs from reeve (plan, "Listing format
//! hall needs from reeve"). DRAFT: the real format is owned by a later reeve
//! change the liege approves (ADR 0002, decision 11). Shape: docs/listing-draft.md.

use serde::Serialize;

use crate::world::{HandState, World};

/// Which herdr session, and which session (pane) inside it, holds a reeve or a hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionLocation {
    pub herdr_session: String,
    /// None until the test setup has recorded the pane.
    pub pane_id: Option<String>,
}

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
    /// Always true: this shape is a draft until reeve owns it.
    pub draft: bool,
    pub reeves: Vec<ReeveEntry>,
    /// Errands in flight only (decision 16).
    pub errands: Vec<ErrandEntry>,
}

impl Listing {
    pub fn from_world(world: &World) -> Listing {
        Listing {
            draft: true,
            reeves: world
                .reeve
                .iter()
                .map(|r| ReeveEntry {
                    name: r.name.clone(),
                    session: SessionLocation {
                        herdr_session: r.herdr_session.clone(),
                        pane_id: r.pane_id.clone(),
                    },
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
                    session: SessionLocation {
                        herdr_session: e.herdr_session.clone(),
                        pane_id: e.pane_id.clone(),
                    },
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
        assert!(l.draft);
        assert_eq!(l.errands.len(), 1);
        assert_eq!(l.errands[0].id, "kept");
        assert_eq!(l.errands[0].question, None);
        let v: serde_json::Value = serde_json::from_str(&l.to_json()).unwrap();
        assert_eq!(v["reeves"][0]["session"]["pane_id"], "w1:p1");
        assert_eq!(v["errands"][0]["hand_state"], "working");
        assert_eq!(
            v["errands"][0]["session"]["pane_id"],
            serde_json::Value::Null
        );
    }
}
