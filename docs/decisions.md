# Hall: decisions settled by grilling

Round 1 (2026-10-12):
1. Core: reeve-first overview. One entry per live reeve, hands nested, hand state from status files, badge for waiting on you; pick a reeve to open its session. (yes)
2. Shape: own full-screen app on herdr (A), not a plugin pane (B). ADR 0001.
3. Home: own repo /Users/yong/yongshq/hall, own manor, name hall. Repo created, empty first commit 0d1fdb4. (yes)
4. Vocabulary: household terms reused; "space" becomes "reeve" at the top level. GLOSSARY.md. (yes)

Round 2 (2026-10-12):
5. Language: Rust. (yes)
6. Scope of reeves: every reeve on this machine, whichever herdr session it sits in. Liege: "should show all reeves, like a lord's audience hall". (yes)
7. Facts: hand state comes from reeve's own commands via a machine-readable mode, never from reading status files. ADR 0002. (yes)
8. Version one: see and talk only. Reeve tree with states and badges, open any session and type into it. Household actions stay with the reeve. (yes)
9. Standing constraint from the liege: "while this is being developed, leave our current reeve setup and herdr setup untouched as it's the current setup we use. Keep it separated".
Dropped: tightening the wildcard permission rules in the liege's Claude settings (answer: no).

Round 3 (2026-10-12), "yes to all":
10. Test setup: hall is built and tested only against its own herdr session (hall-dev, own server and socket) and its own reeve home (~/.reeve-hall-dev) with stand-in reeves. Never the herdr session default or ~/.reeve. ADR 0003.
11. Reeve change deferred: hall defines the listing format it needs and is built against a stand-in serving it from the test home. The real reeve addition goes to the liege as its own change only when hall is ready for the live setup.
12. Layout: left column reeve tree (reeves, hands nested, state mark per hand, count badge for waiting on you), selected session fills the rest, one line of key help at the bottom.
13. Starting: hall is a command run in the liege's terminal; attaches to a herdr server already running; never starts, stops or reconfigures herdr in version one.

Round 4 (2026-10-12), "yes to all":
14. Prefix key: ctrl+b, as in the liege's herdr and tmux, configurable. Prefix then a key for hall's commands; everything else goes to the session.
15. Typing control: hall takes a session's input only while that session is open and focused in hall; hands it back on moving away.
16. Reeve tree shows errands in flight only; a hand drops off once finished and cleaned up.
17. Badge in version one counts only what is on disk: a hand's open question, a hand at a dialog (waiting), blocked, idle or wedged. The reeve's full waiting list comes later as a reeve change the liege approves.
18. Repo stays private and local, no remote, until version one works; publishing decided then.

Round 5 (2026-10-12):
19. Alerts: liege: "I would like for the sound to be kept, atleast for when a hand it stuck or for when a reeve needs anything from me." Version one plays a sound (configurable, can be muted) and highlights the reeve when: a hand becomes stuck (waiting, blocked, idle or wedged) or opens a question; or a reeve's own session stands at a dialog or ends its turn. Reeve's own session state is read from herdr, for alerting only, never as errand state (ADR 0002 unaffected). Hall plays the sound itself, because herdr's own sounds need a herdr window in the foreground.
20. Shared understanding confirmed by the liege; grilling ended. (yes)
21. Planning set committed into hall by a writer, plan document with five end-to-end steps, no code, reviewed after. (yes)

After round 5 (2026-10-12):
22. Alert sound stays at the end of every reeve reply plus every dialog, for version one. Narrow to replies that ask the liege something later, if noisy; that needs a signal reeve does not record today. Liege: "sure, keep it as is for now".

After the plan review (2026-10-12), reeve's calls under the decisions already made:
23. Test setup gets a second herdr session, hall-dev-2, so decision 6 (reeves across herdr sessions) is shown before live use. Same separation rules.
24. Every herdr call in hall's test setup names its herdr session on the command line (--session) or connects to that session's socket path directly; never relies on HERDR_SESSION or an inherited HERDR_SOCKET_PATH, which inside a live pane points at default. Herdr's global config and plugin registry are shared with the live setup: never edited for testing; a test config goes through HERDR_CONFIG_PATH.
25. Glossary gains "Herdr session", "Test reeve" and "Test hand"; "Stand-in" stays the listing substitute only.
Closed by decision 27 (was open with the liege: herdr upgrade 0.9.0 to 0.9.3, which would change the live backend).
26. Every test reeve and test hand session starts with REEVE_HOME=~/.reeve-hall-dev on its own command line, never relying on the default: the liege's reeve plugin hooks run in every claude session and would otherwise write into ~/.reeve. (From the repair review.)

After landing (2026-10-12):
27. Herdr stays at the installed 0.9.0 for all of hall's development; no upgrade (liege: "yes to all"). Closes the open note after decision 25.

After the step 1 review (2026-10-12), liege: "yes to both":
28. The separation rule stays absolute: no herdr call ever names the session default, not even a read. Plan wording "the default session ... as they were" means what is checked: every pane the test setup made is recorded with its hall-dev* session and confirmed there, and default's entry in herdr session list is unchanged against the baseline. Which panes default holds is not checked.
29. Step 1 lands with one low accepted as known: reeve makes a hard link for an instant in two places a test reeve writes, the wake spool (ln then rm under state/sessions/<sid>/wake) and the sentry lock (state/.sentry.lock and state/.sentry.lock.new.<pid>). So up or test-drive can refuse a test home once with HardLink, naming either name of the pair, and pass on rerun. A writer killed mid-write leaves its .new.* name linked: remove only the .new.* name (never the spool entry or the lock it is linked to), or for the wake spool wait for the wake to be delivered. Fail-safe only. Documented in docs/test-setup.md.

After the design check (2026-10-12), liege: "sure, let's go with A":
30. Hall acts and starts like herdr: the liege types `hall` and nothing else. If its herdr backend is not running, hall starts it out of sight, then shows the reeve screen; running `hall` again reattaches. The liege never runs or sees herdr for hall. Replaces decision 13's "never starts herdr". Hall is still built on herdr as it ships (ADR 0001 stands; forking herdr rejected: it would still depend on herdr, and every herdr update becomes a hand merge). Hall should run on its own pinned copy of herdr, so updating the liege's Homebrew herdr never breaks hall; herdr is Apache-2.0 (brew info, research section 6), so a pinned copy is allowed with its licence notice kept. Hall checks herdr's protocol version at start and refuses with a clear message on a mismatch. During development the separation rule (decision 9, ADR 0003) is unchanged: hall starts only hall-dev* sessions, never default.

After the herdr-launch report (2026-10-12), liege: "I think the best way is to just create our own session and multiplexer like maybe fork herdr so any breaking changes from it doesn't affect hall":
31. Hall is a fork of herdr (Apache-2.0), starting from v0.9.0, renamed hall: its own program, its own sessions and its own config and state folders, sharing nothing with the live herdr install. Typed as `hall`, it starts and attaches exactly as herdr does, with reeves at the top level and their hands beneath. Herdr upstream changes come in only when chosen and ported by hand. Supersedes ADR 0001 (own app on herdr) and decision 30's pinned-copy design; decision 30's "type hall and nothing else" stands. Reeves live inside hall once it is in use. Cost accepted with the choice: herdr v0.9.0 is about 241k lines of Rust, and 229 commits landed between v0.9.0 and v0.9.3. The separation rule (decision 9, ADR 0003) is unchanged during development.

After the fork-map report (2026-10-12), reeve's calls under decision 31 (evidence: ~/.reeve/errands/fork-map/report.md):
32. Repo layout: herdr's history merged at hall's root (git merge --allow-unrelated-histories v0.9.0), herdr's crate the root package, crates/hall-standin kept beside it in one cargo workspace. Remote `upstream` = github.com/herdrdev/herdr, fetch only, push URL disabled (decision 18). Herdr's AGENTS.md replaced by hall's own (herdr's kept as a reference file); build.rs's external-contributor warning dropped. Licence: herdr LICENSE and both vendor MIT licences kept, a NOTICE line naming hall as a modified herdr, modified files listed in MODIFICATIONS.md, no herdr branding.
33. Separation in the fork (ADR 0003 holds, mechanism changes): config and state dirs named hall (app_dir_name), binary `hall`, inbound env renamed HALL_*, protocol number bumped off 22 so herdr and hall refuse each other loudly, update/self-replace, agent-manifest fetch, --remote install and integration install refused, worktree and pane-graphics paths renamed. Replaces decision 24's mechanism for hall itself; the test setup makes no herdr call at all.
34. Reeve tree shape: reeve = workspace, hand = tab, as reeve's herdr backend already lays them out; workspaces that are not reeves kept apart from the reeve list. Client-side sidebar change only.
35. Sound: hall reuses herdr's own request and done sounds, which play in the hall window (decision 19's reason for a separate player no longer holds). Hall adds the triggers herdr cannot see: a hand blocked, idle, wedged or asking, from reeve's state. Decision 22 (every reeve reply) kept: if herdr's suppression for the focused active tab hides it, a setting turns that off.
36. The reeve side of the bridge (a hall backend in reeve, and whether reeve pushes hand state to hall or hall reads the listing) is decided with the liege at go-live, with decision 11. Until then test reeves do not dispatch through hall; the test setup places test hands itself.
37. Build: Rust 1.96.1 (herdr's pin) and zig 0.15.2. This Mac's SDK needs a workaround for zig 0.15.2 (fork-map section 6); installing tools is the liege's call.

After the fork-plan review (2026-10-12), liege: "yes to all":
38. Hall's reeve tree shows one hall session: once hall is in use every reeve lives in hall's own session (decision 31), so nothing needs combining. Decision 6 ("every reeve on the machine, whichever herdr session") is scoped to that one hall session; decision 23's second test session hall-dev-2 is dropped. During the switch-over, reeves still in the live herdr stay visible in herdr, not in hall.
39. The liege approved installing the build tools: zig 0.15 from Homebrew and Rust 1.96.1 through rustup beside the existing toolchain. If Homebrew's zig still fails to link on this Mac, the SDK workaround lives in hall's own build scripts, with nothing further installed.
