//! The stand-in binary end to end, against temp test reeve homes under target/.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const WORLD: &str = r#"
[[reeve]]
name = "ansel"
herdr_session = "hall-dev"
pane_id = "w1:p1"

[[reeve]]
name = "bertil"
herdr_session = "hall-dev-2"
live = false

[[errand]]
id = "ansel-fix"
office = "artificer"
owner = "ansel"
herdr_session = "hall-dev"
pane_id = "w2:p1"

[[errand]]
id = "bertil-probe"
office = "warden"
owner = "bertil"
state = "needs-decision"
question = "Which base branch?"
herdr_session = "hall-dev-2"

[[errand]]
id = "ansel-old"
office = "artificer"
owner = "ansel"
state = "done"
in_flight = false
herdr_session = "hall-dev"
"#;

/// A fresh test reeve home named .reeve-hall-dev under target/, holding `world`.
fn home(test: &str, world: &str) -> PathBuf {
    let h = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("standin-cli")
        .join(test)
        .join(".reeve-hall-dev");
    let _ = std::fs::remove_dir_all(&h);
    std::fs::create_dir_all(h.join("standin")).unwrap();
    std::fs::write(h.join("standin/world.toml"), world).unwrap();
    h
}

fn standin(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hall-standin"))
        .args(args)
        .output()
        .unwrap()
}

fn ok(args: &[&str]) -> String {
    let out = standin(args);
    assert!(
        out.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn refused(out: &Output, code: i32, needle: &str) {
    assert_eq!(out.status.code(), Some(code), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains(needle), "stderr {err:?} lacks {needle:?}");
}

fn listing(h: &Path) -> Value {
    serde_json::from_str(&ok(&["--home", h.to_str().unwrap()])).unwrap()
}

fn errand<'a>(l: &'a Value, id: &str) -> &'a Value {
    l["errands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id)
        .unwrap_or_else(|| panic!("errand {id} not listed"))
}

fn keys(v: &Value) -> Vec<&str> {
    let mut k: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    k.sort();
    k
}

#[test]
fn listing_shape() {
    let h = home("shape", WORLD);
    let l = listing(&h);
    assert_eq!(keys(&l), ["errands", "reeves", "version"]);
    assert_eq!(l["version"], 1);

    let reeves = l["reeves"].as_array().unwrap();
    assert_eq!(reeves.len(), 2);
    assert_eq!(keys(&reeves[0]), ["live", "name", "session"]);
    assert_eq!(keys(&reeves[0]["session"]), ["backend", "name", "pane_id"]);
    assert_eq!(reeves[0]["session"]["backend"], "hall");
    assert_eq!(reeves[0]["name"], "ansel");
    assert_eq!(reeves[0]["session"]["name"], "hall-dev");
    assert_eq!(reeves[0]["session"]["pane_id"], "w1:p1");
    assert_eq!(reeves[0]["live"], true);
    assert_eq!(reeves[1]["live"], false);
    assert_eq!(reeves[1]["session"]["pane_id"], Value::Null);

    let e = errand(&l, "ansel-fix");
    assert_eq!(
        keys(e),
        [
            "hand_state",
            "id",
            "idle",
            "office",
            "owner_reeve",
            "question",
            "session",
            "waiting",
            "wedged"
        ]
    );
    assert_eq!(e["office"], "artificer");
    assert_eq!(e["owner_reeve"], "ansel");
    assert_eq!(e["hand_state"], "working");
    assert_eq!(e["session"]["backend"], "hall");
    assert_eq!(e["session"]["name"], "hall-dev");
    assert_eq!(e["session"]["pane_id"], "w2:p1");
    for flag in ["waiting", "idle", "wedged"] {
        assert_eq!(e[flag], false, "{flag}");
    }
}

#[test]
fn errands_not_in_flight_are_omitted() {
    let h = home("in-flight", WORLD);
    let l = listing(&h);
    let ids: Vec<&str> = l["errands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["ansel-fix", "bertil-probe"]);

    let hs = h.to_str().unwrap();
    ok(&["--home", hs, "set", "ansel-fix", "in_flight=false"]);
    assert_eq!(listing(&h)["errands"].as_array().unwrap().len(), 1);
    ok(&["--home", hs, "set", "ansel-old", "in_flight=true"]);
    assert_eq!(errand(&listing(&h), "ansel-old")["hand_state"], "done");
}

#[test]
fn every_hand_state() {
    let h = home("states", WORLD);
    let hs = h.to_str().unwrap();
    for state in ["working", "needs-decision", "blocked", "done", "failed"] {
        ok(&["--home", hs, "set", "ansel-fix", &format!("state={state}")]);
        assert_eq!(errand(&listing(&h), "ansel-fix")["hand_state"], state);
    }
    let out = standin(&["--home", hs, "set", "ansel-fix", "state=stuck"]);
    refused(&out, 1, "unknown hand state stuck");
}

#[test]
fn every_flag() {
    let h = home("flags", WORLD);
    let hs = h.to_str().unwrap();
    for flag in ["waiting", "idle", "wedged"] {
        ok(&["--home", hs, "set", "ansel-fix", &format!("{flag}=true")]);
        let e = errand(&listing(&h), "ansel-fix").clone();
        assert_eq!(e[flag], true, "{flag}");
        for other in ["waiting", "idle", "wedged"].iter().filter(|o| **o != flag) {
            assert_eq!(e[*other], false, "{flag} set, {other} should not be");
        }
        ok(&["--home", hs, "set", "ansel-fix", &format!("{flag}=false")]);
        assert_eq!(errand(&listing(&h), "ansel-fix")[flag], false);
    }
    // Editing the file by hand works the same way as set.
    let h = home("flags-by-hand", WORLD);
    let path = h.join("standin/world.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("pane_id = \"w2:p1\"", "pane_id = \"w2:p1\"\nwedged = true");
    std::fs::write(&path, text).unwrap();
    assert_eq!(errand(&listing(&h), "ansel-fix")["wedged"], true);
}

#[test]
fn question_present_and_absent() {
    let h = home("question", WORLD);
    let hs = h.to_str().unwrap();
    let l = listing(&h);
    assert_eq!(errand(&l, "bertil-probe")["question"], "Which base branch?");
    assert_eq!(errand(&l, "ansel-fix")["question"], Value::Null);

    ok(&[
        "--home",
        hs,
        "set",
        "ansel-fix",
        "question=Merge or rebase?",
    ]);
    assert_eq!(
        errand(&listing(&h), "ansel-fix")["question"],
        "Merge or rebase?"
    );
    ok(&["--home", hs, "set", "bertil-probe", "question="]);
    assert_eq!(
        errand(&listing(&h), "bertil-probe")["question"],
        Value::Null
    );
}

#[test]
fn reeve_live_and_place() {
    let h = home("place", WORLD);
    let hs = h.to_str().unwrap();
    ok(&["--home", hs, "set", "bertil", "live=true"]);
    ok(&["--home", hs, "place", "bertil", "hall-dev-2", "w3:p1"]);
    let l = listing(&h);
    assert_eq!(l["reeves"][1]["live"], true);
    assert_eq!(l["reeves"][1]["session"]["pane_id"], "w3:p1");
    assert_eq!(
        ok(&["--home", hs, "locate", "bertil"]),
        "hall-dev-2 w3:p1\n"
    );

    ok(&["--home", hs, "place", "bertil", "hall-dev-2"]);
    assert_eq!(listing(&h)["reeves"][1]["session"]["pane_id"], Value::Null);
    refused(
        &standin(&["--home", hs, "locate", "bertil"]),
        1,
        "no pane recorded",
    );
    // has: named, with a pane or not.
    assert_eq!(ok(&["--home", hs, "has", "bertil"]), "");
    refused(
        &standin(&["--home", hs, "has", "nobody"]),
        1,
        "no reeve or errand called nobody",
    );

    let sessions = ok(&["--home", hs, "sessions"]);
    assert_eq!(
        sessions,
        "reeve ansel hall-dev w1:p1\nreeve bertil hall-dev-2 -\n\
         errand ansel-fix hall-dev w2:p1\nerrand bertil-probe hall-dev-2 -\n"
    );
}

#[test]
fn refuses_missing_home() {
    refused(&standin(&[]), 2, "--home <test reeve home> is required");
    refused(&standin(&["listing"]), 2, "there is no default");
    refused(&standin(&["--home"]), 2, "--home needs a path");
}

#[test]
fn refuses_the_live_reeve_home() {
    // The liege's real ~/.reeve: refused before anything is read.
    let real = std::env::var("HOME").unwrap();
    let live = format!("{real}/.reeve");
    refused(&standin(&["--home", &live]), 1, "live reeve home");
    refused(
        &standin(&["--home", &format!("{live}/.reeve-hall-dev")]),
        1,
        "live reeve home",
    );

    // A fake HOME under target/, so a .reeve that exists there is ruled out too,
    // including through a symlink named like a test home.
    let fake = Path::new(env!("CARGO_TARGET_TMPDIR")).join("standin-cli/fake-user");
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(fake.join(".reeve/standin")).unwrap();
    std::fs::write(fake.join(".reeve/standin/world.toml"), WORLD).unwrap();
    std::os::unix::fs::symlink(fake.join(".reeve"), fake.join(".reeve-hall-dev")).unwrap();
    for arg in [fake.join(".reeve"), fake.join(".reeve-hall-dev")] {
        let out = Command::new(env!("CARGO_BIN_EXE_hall-standin"))
            .env("HOME", &fake)
            .args(["--home", arg.to_str().unwrap()])
            .output()
            .unwrap();
        refused(&out, 1, "live reeve home");
    }

    let out = Command::new(env!("CARGO_BIN_EXE_hall-standin"))
        .env_remove("HOME")
        .args(["--home", "/nowhere/.reeve-hall-dev"])
        .output()
        .unwrap();
    refused(&out, 1, "HOME is not set");
}

#[test]
fn refuses_the_live_reeve_home_in_any_letter_case() {
    let real = std::env::var("HOME").unwrap();
    for h in [
        format!("{real}/.REEVE"),
        format!("{real}/.Reeve/.reeve-hall-dev"),
    ] {
        refused(
            &standin(&["--home", &h, "check-home"]),
            1,
            "live reeve home",
        );
    }

    // A fake HOME whose .reeve exists, and one whose .reeve does not.
    for with_reeve in [true, false] {
        let fake = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("standin-cli/case-user-{with_reeve}"));
        let _ = std::fs::remove_dir_all(&fake);
        std::fs::create_dir_all(&fake).unwrap();
        if with_reeve {
            std::fs::create_dir_all(fake.join(".reeve")).unwrap();
        }
        for arg in [fake.join(".REEVE/.reeve-hall-dev"), fake.join(".ReEvE")] {
            let out = Command::new(env!("CARGO_BIN_EXE_hall-standin"))
                .env("HOME", &fake)
                .args(["--home", arg.to_str().unwrap(), "check-home"])
                .output()
                .unwrap();
            refused(&out, 1, "live reeve home");
        }
    }
}

#[test]
fn refuses_the_user_database_home_under_a_wrong_home() {
    let passwd = hall_standin::home::passwd_home().unwrap();
    let fake = Path::new(env!("CARGO_TARGET_TMPDIR")).join("standin-cli/wrong-home");
    std::fs::create_dir_all(&fake).unwrap();
    for arg in [passwd.join(".reeve"), passwd.join(".reeve/.reeve-hall-dev")] {
        let out = Command::new(env!("CARGO_BIN_EXE_hall-standin"))
            .env("HOME", &fake)
            .args(["--home", arg.to_str().unwrap(), "check-home"])
            .output()
            .unwrap();
        refused(&out, 1, "live reeve home");
    }
}

#[test]
fn check_home_prints_the_home_as_resolved() {
    let h = home("check-home", WORLD);
    let parent = h.parent().unwrap();
    let link = parent.join(".reeve-hall-dev-link");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(&h, &link).unwrap();
    let want = format!("{}\n", h.canonicalize().unwrap().display());
    assert_eq!(ok(&["--home", h.to_str().unwrap(), "check-home"]), want);
    assert_eq!(ok(&["check-home", "--home", link.to_str().unwrap()]), want);
    // A home that does not exist yet is checked all the same.
    let new = parent.join("new/.reeve-hall-dev");
    assert_eq!(
        ok(&["--home", new.to_str().unwrap(), "check-home"]),
        format!(
            "{}\n",
            parent
                .canonicalize()
                .unwrap()
                .join("new/.reeve-hall-dev")
                .display()
        )
    );
}

/// Runs the stand-in with HOME set to `user`.
fn standin_as(user: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hall-standin"))
        .env("HOME", user)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn refuses_a_symlink_inside_the_home() {
    // A fake HOME whose live reeve home holds a world file and a baseline.
    let fake = Path::new(env!("CARGO_TARGET_TMPDIR")).join("standin-cli/inner-user");
    let _ = std::fs::remove_dir_all(&fake);
    let live = fake.join(".reeve");
    std::fs::create_dir_all(live.join("work")).unwrap();
    std::fs::write(live.join("world.toml"), WORLD).unwrap();
    std::fs::write(live.join("live-baseline.json"), "{}").unwrap();
    let before = std::fs::read_to_string(live.join("world.toml")).unwrap();

    let h = fake.join(".reeve-hall-dev");
    let hs = h.to_str().unwrap();
    let plants: [(&str, PathBuf); 9] = [
        ("standin", live.clone()),
        ("work", live.clone()),
        ("work/ansel", live.clone()),
        // What reeve writes under REEVE_HOME, and claude in its working directory.
        ("state", live.clone()),
        ("errands", live.join("errands")),
        ("work/ansel/.claude", live.clone()),
        ("standin/world.toml", live.join("world.toml")),
        (
            "standin/live-baseline.json",
            live.join("live-baseline.json"),
        ),
        ("standin/world.toml.tmp", live.join("world.toml")),
    ];
    let commands: [&[&str]; 5] = [
        &["--home", hs, "check-home"],
        &["--home", hs],
        &["--home", hs, "set", "ansel", "live=false"],
        &["--home", hs, "place", "ansel", "hall-dev", "w9:p9"],
        &["--home", hs, "locate", "ansel"],
    ];
    for (name, target) in plants {
        let _ = std::fs::remove_dir_all(&h);
        std::fs::create_dir_all(&h).unwrap();
        for d in ["standin", "work"].into_iter().filter(|d| *d != name) {
            std::fs::create_dir(h.join(d)).unwrap();
        }
        if name != "standin" && name != "standin/world.toml" {
            std::fs::write(h.join("standin/world.toml"), WORLD).unwrap();
        }
        std::fs::create_dir_all(h.join(name).parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&target, h.join(name)).unwrap();
        for args in commands {
            refused(&standin_as(&fake, args), 1, "is a symlink");
        }
    }
    // No write landed in the live reeve home.
    assert_eq!(
        std::fs::read_to_string(live.join("world.toml")).unwrap(),
        before
    );
    assert_eq!(
        std::fs::read_to_string(live.join("live-baseline.json")).unwrap(),
        "{}"
    );
    let mut names: Vec<_> = std::fs::read_dir(&live)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    names.sort();
    assert_eq!(names, ["live-baseline.json", "work", "world.toml"]);
}

#[test]
fn refuses_a_home_it_cannot_list() {
    let fake = Path::new(env!("CARGO_TARGET_TMPDIR")).join("standin-cli/unlisted-user");
    let h = fake.join(".reeve-hall-dev");
    let open = |d: &str| {
        let _ = std::fs::set_permissions(h.join(d), std::fs::Permissions::from_mode(0o755));
    };
    for dir in ["standin", "work"] {
        open("standin");
        open("work");
        let _ = std::fs::remove_dir_all(&fake);
        let live = fake.join(".reeve");
        std::fs::create_dir_all(&live).unwrap();
        std::fs::create_dir_all(h.join("standin")).unwrap();
        std::fs::create_dir_all(h.join("work")).unwrap();
        // Search but no read: the link inside can be followed but not listed.
        if dir == "standin" {
            std::os::unix::fs::symlink(live.join("world.toml"), h.join("standin/world.toml"))
                .unwrap();
        } else {
            std::os::unix::fs::symlink(&live, h.join("work/ansel")).unwrap();
        }
        std::fs::set_permissions(h.join(dir), std::fs::Permissions::from_mode(0o300)).unwrap();
        let hs = h.to_str().unwrap();
        for args in [&["--home", hs, "check-home"][..], &["--home", hs]] {
            refused(
                &standin_as(&fake, args),
                1,
                "cannot be checked for symlinks",
            );
        }
        open(dir);
        assert!(!live.join("world.toml").exists());
    }
}

#[test]
fn refuses_a_home_past_the_walk_limit() {
    let h = home("walk-limit", WORLD);
    let deep: PathBuf = std::iter::repeat_n("d", hall_standin::home::MAX_DEPTH + 1).collect();
    std::fs::create_dir_all(h.join(&deep)).unwrap();
    let hs = h.to_str().unwrap();
    refused(
        &standin(&["--home", hs, "check-home"]),
        1,
        "past what the symlink check walks",
    );
    // One level less is walked in full.
    std::fs::remove_dir(h.join(&deep)).unwrap();
    ok(&["--home", hs, "check-home"]);
}

#[test]
fn refuses_a_hard_link_inside_the_home() {
    let fake = Path::new(env!("CARGO_TARGET_TMPDIR")).join("standin-cli/hard-link-user");
    let _ = std::fs::remove_dir_all(&fake);
    let live = fake.join(".reeve");
    std::fs::create_dir_all(live.join("state")).unwrap();
    std::fs::write(live.join("state/sentry.log"), "live\n").unwrap();
    let h = fake.join(".reeve-hall-dev");
    std::fs::create_dir_all(h.join("standin")).unwrap();
    std::fs::create_dir_all(h.join("state")).unwrap();
    std::fs::write(h.join("standin/world.toml"), WORLD).unwrap();
    std::fs::hard_link(live.join("state/sentry.log"), h.join("state/sentry.log")).unwrap();
    let hs = h.to_str().unwrap();
    let commands: [&[&str]; 4] = [
        &["--home", hs, "check-home"],
        &["--home", hs],
        &["--home", hs, "set", "ansel", "live=false"],
        &["--home", hs, "place", "ansel", "hall-dev", "w9:p9"],
    ];
    for args in commands {
        let out = standin_as(&fake, args);
        refused(&out, 1, "state/sentry.log is a hard link");
    }
    // The root check alone passes it, as down and status need.
    assert_eq!(
        String::from_utf8(standin_as(&fake, &["--home", hs, "check-root"]).stdout).unwrap(),
        format!("{}\n", h.canonicalize().unwrap().display())
    );
    assert_eq!(
        std::fs::read_to_string(live.join("state/sentry.log")).unwrap(),
        "live\n"
    );
}

#[test]
fn refuses_a_fifo_inside_the_home() {
    let h = home("fifo", WORLD);
    let world = h.join("standin/world.toml");
    std::fs::remove_file(&world).unwrap();
    let status = Command::new("mkfifo").arg(&world).status().unwrap();
    assert!(status.success());
    let hs = h.to_str().unwrap();
    // check-home first: it reads nothing, so without the refusal it passes at
    // once, where the listing would hang reading the fifo.
    let commands: [&[&str]; 4] = [
        &["--home", hs, "check-home"],
        &["--home", hs],
        &["--home", hs, "set", "ansel", "live=false"],
        &["--home", hs, "locate", "ansel"],
    ];
    for args in commands {
        refused(
            &standin(args),
            1,
            "is neither a regular file nor a directory",
        );
    }
}

#[test]
fn a_leftover_temp_file_is_replaced_not_followed() {
    let h = home("leftover-tmp", WORLD);
    std::fs::write(h.join("standin/world.toml.tmp"), "half written").unwrap();
    ok(&["--home", h.to_str().unwrap(), "set", "ansel", "live=false"]);
    assert_eq!(listing(&h)["reeves"][0]["live"], false);
    assert!(!h.join("standin/world.toml.tmp").exists());
}

#[test]
fn refuses_a_parent_dir_in_the_home() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("standin-cli/parent-dir");
    let _ = std::fs::remove_dir_all(&dir);
    // Spelled out, link/.. is `dir`; the kernel follows the link first and
    // lands in `elsewhere`, so the two readings differ.
    std::fs::create_dir_all(dir.join("elsewhere/inner")).unwrap();
    std::os::unix::fs::symlink(dir.join("elsewhere/inner"), dir.join("link")).unwrap();
    let arg = dir.join("link/../.reeve-hall-dev");
    let a = arg.to_str().unwrap();
    let commands: [&[&str]; 3] = [
        &["--home", a, "check-home"],
        &["--home", a],
        &["--home", "link/../.reeve-hall-dev", "check-home"],
    ];
    for args in commands {
        refused(&standin(args), 1, "no .. in the path");
    }
    assert!(!dir.join(".reeve-hall-dev").exists());
    assert!(!dir.join("elsewhere/.reeve-hall-dev").exists());
}

#[test]
fn refuses_pane_ids_that_read_as_options() {
    let h = home(
        "bad-pane",
        &WORLD.replace("pane_id = \"w1:p1\"", "pane_id = \"--session=default\""),
    );
    refused(&standin(&["--home", h.to_str().unwrap()]), 1, "pane id");

    let h = home("bad-place", WORLD);
    let hs = h.to_str().unwrap();
    for bad in ["-w1", "w1 p1", "w1;p1"] {
        refused(
            &standin(&["--home", hs, "place", "ansel", "hall-dev", bad]),
            1,
            "pane id",
        );
    }
    assert_eq!(ok(&["--home", hs, "locate", "ansel"]), "hall-dev w1:p1\n");
}

#[test]
fn refuses_a_home_not_named_as_a_test_home() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("standin-cli/other-home");
    refused(
        &standin(&["--home", dir.to_str().unwrap()]),
        1,
        "starts with .reeve-hall-dev",
    );
}

#[test]
fn refuses_herdr_session_default() {
    let h = home(
        "default-session",
        &WORLD.replace(
            "herdr_session = \"hall-dev-2\"\nlive",
            "herdr_session = \"default\"\nlive",
        ),
    );
    refused(
        &standin(&["--home", h.to_str().unwrap()]),
        1,
        "belongs to the live setup",
    );

    let h = home("default-place", WORLD);
    let hs = h.to_str().unwrap();
    refused(
        &standin(&["--home", hs, "place", "ansel-fix", "default", "w1:p9"]),
        1,
        "belongs to the live setup",
    );
    // The refused edit left the world file as it was.
    assert_eq!(
        errand(&listing(&h), "ansel-fix")["session"]["name"],
        "hall-dev"
    );
}
