# Hall

Hall is the liege's audience hall: one screen over every live reeve on the machine, each reeve at the top level, its hands beneath, and what is waiting on the liege at a glance. Its language is the reeve household's own, so the two never drift.

## People and agents

**Liege**:
The one human hall serves, and the only person any reeve addresses.
_Avoid_: user, operator

**Reeve**:
One managing agent, running, that dispatches hands on the liege's behalf. The top-level unit in hall.
_Avoid_: space, workspace, project

**Hand**:
A worker agent a reeve dispatched for one errand. Always shown beneath the reeve that owns it.
_Avoid_: worker, sub-agent, child

**Errand**:
One unit of work a reeve sent out: an id, its instructions, a local copy and a status file.
_Avoid_: task, job, ticket

## Where things run

**Session**:
One running reeve or hand, together with the terminal it lives in. Picking a reeve or a hand in hall shows its session.
_Avoid_: pane, tab, endpoint, window

**Herdr session**:
One herdr server with its own socket, holding many sessions: `default` for the live setup, `hall-dev` and `hall-dev-2` for the test setup.
_Avoid_: server, instance, session (alone)

**Backend**:
The terminal manager that keeps every session alive underneath hall. Today, herdr.
_Avoid_: engine, multiplexer, server

**Local copy**:
The separate checkout of a repository a hand works in, so hands never collide.
_Avoid_: worktree, clone

## Setups

**Live setup**:
The liege's real reeves, reeve home and herdr session, doing the household's actual work.
_Avoid_: production, prod, real setup

**Test setup**:
A separate herdr session and reeve home, holding stand-in reeves, that hall is built and tested against.
_Avoid_: sandbox, staging, dev env

**Test reeve**:
An agent session in the test setup that plays a reeve, so hall has something real to show and alert on.
_Avoid_: fake reeve, mock reeve, stand-in

**Test hand**:
An agent session in the test setup that plays a hand under a test reeve.
_Avoid_: fake hand, mock hand, stand-in

**Stand-in**:
A substitute for reeve's listing that serves the format hall needs from the test setup.
_Avoid_: mock, fake, stub

## Screen

**Reeve tree**:
The left column of hall: every reeve, its hands nested beneath, a state mark per hand and a waiting-on-you badge per reeve.
_Avoid_: sidebar, agent list

## State

**In flight**:
An errand not yet finished and cleaned up. Only errands in flight appear in the reeve tree.
_Avoid_: active, open, running

**Hand state**:
Where an errand stands by its own status file, as the reeve reconciles it: working, needs a decision, blocked, done or failed. Never read from the screen.
_Avoid_: agent status, pane status

**Waiting**:
A permission dialog stands in a hand's session and the hand cannot report it.
_Avoid_: blocked (herdr uses "blocked" for this; in hall, blocked means the next term)

**Blocked**:
A hand has reported that something stands in its way.
_Avoid_: stuck, waiting

**Idle**:
A hand whose turn died: its session sits quiet and its status file has said nothing for too long.
_Avoid_: asleep, done

**Wedged**:
A hand stuck inside one command: working, yet neither its status file nor its screen has moved for too long.
_Avoid_: hung, frozen

**Stuck**:
A hand that cannot go on without help: waiting, blocked, idle or wedged. Always alerts the liege.
_Avoid_: hung, stalled

**Waiting on you**:
Anything a reeve has put to the liege and the liege has not yet answered. Hall shows a badge for it on that reeve.
_Avoid_: pending, notification, inbox

**Alert**:
The sound hall plays, with a highlight on the reeve, when a hand becomes stuck or a reeve needs the liege.
_Avoid_: notification, toast, ping
