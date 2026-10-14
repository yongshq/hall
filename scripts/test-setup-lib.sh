# shellcheck shell=bash
# Shared by scripts/test-setup and scripts/test-drive. Sourced, never run.
#
# The rules every function here keeps (ADR 0003, decisions 26, 33, 38 and 47):
# - no herdr call at all: every call runs the debug hall binary, names the one
#   hall session with --session on the command line, and runs under `env -u` of
#   the HALL_* names hall reads or sets into its panes and of REEVE_BACKEND,
#   because a process inside a hall pane inherits HALL_SESSION, HALL_PANE_ID,
#   HALL_SOCKET_PATH, REEVE_BACKEND=hall and the rest;
# - the hall session default and the reeve home ~/.reeve are refused, the home
#   by the stand-in's check-root before anything else happens;
# - every test reeve and test hand claude starts with REEVE_HOME=<test home>,
#   XDG_CONFIG_HOME=<test home>/xdg-config and XDG_STATE_HOME=<test
#   home>/xdg-state on its own command line, so no herdr call from a test pane
#   can find the live herdr's config, state, sessions or sockets; and with
#   HALL_SOCKET_PATH=<the hall session's socket>, so a hall call from a test
#   pane still reaches that session;
# - the hall server up starts carries REEVE_HOME=<test home>, so a hand a test
#   reeve dispatches, whose tab has the server's environment and whose launch
#   line is reeve's, writes under the test home too (it has no XDG guard: see
#   docs/test-setup.md, What the guard costs); so does the attach line up
#   prints, for a server the hall client starts itself, and up refuses a
#   running hall server that does not carry it;
# - nothing about a test claude's identity is inherited from whoever ran the
#   script (decision 48): the hall server up starts drops the names in
#   STARTER_DROP, every claude line drops CLAUDE_DROP's names in the pane's own
#   shell, and a test hand's REEVE_HAND is set on purpose;
# - --dry-run prints every command and runs none.

HALL_REPO=$(cd -P "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
HALL_MODEL=claude-haiku-4-5-20251001
HALL_RUSTUP_BIN=/opt/homebrew/opt/rustup/bin
HALL_ZIG_BIN=/opt/homebrew/opt/zig@0.15/bin
# A prebuilt stand-in may be named here (the tests do); otherwise up builds it.
HALL_STANDIN="${HALL_STANDIN_BIN:-$HALL_REPO/target/debug/hall-standin}"
# A prebuilt hall may be named here (the tests name a fake one); otherwise up
# builds the debug hall, whose folders are ~/.config/hall-dev and the rest.
HALL_BIN="${HALL_DEBUG_BIN:-$HALL_REPO/target/debug/hall}"
# The HALL_* names hall reads or sets into its panes, and the reeve backend it
# sets there (decision 68), dropped from every hall call and build: the test
# setup is run from inside live hall panes too, whose values name the live hall.
HALL_CLEAN_ENV=(env -u HALL_SOCKET_PATH -u HALL_CLIENT_SOCKET_PATH -u HALL_SESSION -u HALL_CONFIG_PATH -u HALL_ENV -u HALL_PANE_ID -u HALL_BIN_PATH -u HALL_TAB_ID -u HALL_WORKSPACE_ID -u REEVE_BACKEND)
HALL_CLI=("${HALL_CLEAN_ENV[@]}" "$HALL_BIN")

# The names in this script's environment that say who a claude is, or which
# herdr or hall it talks to (decision 48): REEVE_HAND, which makes reeve's hooks
# take a test reeve for a hand; reeve's hall backend overrides, which it reads
# before HALL_SOCKET_PATH and HALL_SESSION; claude's own session markers, among
# them its messaging socket and token; and every HERDR_* name, the live herdr's
# socket and session. Dropped from the hall server up starts.
STARTER_FIXED=(REEVE_HAND REEVE_HALL_SOCKET REEVE_HALL_SESSION REEVE_HALL_BIN CLAUDECODE CLAUDE_PID CLAUDE_EFFORT)
STARTER_DROP=("${STARTER_FIXED[@]}")
while read -r __name; do
  case $__name in CLAUDE_CODE_* | HERDR_*) STARTER_DROP+=("$__name") ;; esac
done < <(compgen -e | LC_ALL=C sort)
unset __name
# env -u of every STARTER_DROP name, for the hall server's start.
SERVER_DROP=()
for __name in "${STARTER_DROP[@]}"; do
  SERVER_DROP+=(-u "$__name")
done
unset __name
# The env -u list of the attach line up prints, as shell text. With no hall-dev
# server listening, the hall client starts one itself, with the client's own
# environment (src/server/autodetect.rs), so the line drops what the server's
# start drops: the STARTER_FIXED names, and every CLAUDE_CODE_* and HERDR_* name
# the shell that runs the line holds, worked out there, since whoever attaches
# may hold names the runner of up lacked. command keeps aliases out of it.
ATTACH_DROP=""
for __name in "${STARTER_FIXED[@]}"; do
  ATTACH_DROP+="-u $__name "
done
unset __name
ATTACH_DROP+='$(command env | command sed -nE "s/^((CLAUDE_CODE|HERDR)_[A-Za-z0-9_]*)=.*/-u \1/p")'
# The env -u list of a claude line (decision 48 and its second amendment). The
# pane's own shell works it out when the line runs, from what that shell holds,
# so a hall-dev server someone else started, with names the runner of up lacks,
# passes none of them on either: REEVE_HAND, reeve's hall backend overrides
# (REEVE_HALL_SOCKET, REEVE_HALL_SESSION, REEVE_HALL_BIN, which it reads before
# hall's own names, so one would send the test reeve's dispatches elsewhere),
# claude's session markers, every CLAUDE_CODE_* name and every HERDR_* name
# except the six hall sets into every pane with that session's own values (its
# socket, binary and pane ids), which
# claude's herdr agent-state hook uses to report to hall-dev. Every HALL_* name
# and REEVE_BACKEND are kept too: hall sets them into every pane with that
# session's own values, and reeve's hall backend reads them (decision 48's
# second amendment, decision 68). command keeps the liege's aliases for env and
# sed out of it.
CLAUDE_DROP='-u REEVE_HAND -u REEVE_HALL_SOCKET -u REEVE_HALL_SESSION -u REEVE_HALL_BIN -u CLAUDECODE -u CLAUDE_PID -u CLAUDE_EFFORT $(command env | command sed -nE "/^HERDR_(ENV|SOCKET_PATH|BIN_PATH|WORKSPACE_ID|TAB_ID|PANE_ID)=/d;s/^((CLAUDE_CODE|HERDR)_[A-Za-z0-9_]*)=.*/-u \1/p")'

DRY_RUN=0
CONFIG="$HALL_REPO/scripts/test-setup.conf"
ARG_HOME=""
ARG_SESSION=""
ARG_LISTING=""
# The hall session's API socket, from hall's session list; the test claudes'
# HALL_SOCKET_PATH. Set by up.
HALL_DEV_SOCKET=""

die() {
  printf '%s: %s\n' "$HALL_PROG" "$*" >&2
  exit 1
}

usage_die() {
  printf '%s: %s\n\n%s\n' "$HALL_PROG" "$*" "$HALL_USAGE" >&2
  exit 2
}

# q <args...>: the args as one shell line, quoted only where needed.
q() {
  local out="" a
  for a in "$@"; do
    if [[ $a =~ ^[A-Za-z0-9_./:=@%+,-]+$ ]]; then
      out+="${out:+ }$a"
    else
      out+="${out:+ }'${a//\'/\'\\\'\'}'"
    fi
  done
  printf '%s' "$out"
}

say() { printf '%s\n' "$*"; }

# run <cmd...>: print the command, then run it unless this is a dry run.
run() {
  say "+ $(q "$@")"
  [ "$DRY_RUN" = 1 ] && return 0
  "$@"
}

# detach <cmd...>: print the command, then, unless this is a dry run, start it
# in a session of its own (setsid) with stdio on /dev/null, as hall's own launch
# starts its server, so it outlives this script and the terminal it ran in.
detach() {
  say "+ $(q "$@") &"
  [ "$DRY_RUN" = 1 ] && return 0
  perl -MPOSIX=setsid -e 'setsid() != -1 or die "setsid: $!\n"; exec @ARGV or die "exec $ARGV[0]: $!\n"' "$@" </dev/null >/dev/null 2>&1 &
}

# run_into <var> <placeholder> <cmd...>: like run, keeping stdout in <var>. A dry
# run keeps <placeholder> instead.
run_into() {
  local __var=$1 __placeholder=$2 __out
  shift 2
  say "+ $(q "$@")"
  if [ "$DRY_RUN" = 1 ]; then
    printf -v "$__var" '%s' "$__placeholder"
    return 0
  fi
  __out=$("$@") || return $?
  printf -v "$__var" '%s' "$__out"
}

# no_link <path>: refuses <path> under the test reeve home if it, or any
# directory between the home and it, is a symlink, dangling or not, since a write
# or mkdir would follow it out of the home. The stand-in's check-home refuses
# these too; a write does not lean on that alone.
no_link() {
  local p=$1
  while [ "$p" != "$HALL_DEV_HOME" ] && [ "$p" != / ] && [ "$p" != . ]; do
    if [ -L "$p" ]; then
      die "refusing to write $1: $p is a symlink, which could lead out of the test reeve home"
    fi
    p=$(dirname "$p")
  done
}

# write_file <path> <text>: print what is written, then write it unless this is a dry run.
write_file() {
  no_link "$1"
  say "+ write $(q "$1")"
  [ "$DRY_RUN" = 1 ] && return 0
  printf '%s' "$2" >"$1"
}

# make_dir <path>: mkdir -p, through run.
make_dir() {
  no_link "$1"
  run mkdir -p "$1"
}

check_session() {
  local s=$1
  shopt -s nocasematch
  if [[ $s == default ]]; then
    shopt -u nocasematch
    die "refusing hall session $s: the default session is never the test setup's; it is the live one once hall is in use"
  fi
  shopt -u nocasematch
  [[ $s =~ ^hall-dev[a-z0-9-]*$ ]] || die "refusing hall session $s: test hall sessions are named hall-dev*"
}

# hd <hall session> <hall args...>: one hall call against that session.
hd() {
  local s=$1
  shift
  check_session "$s"
  run "${HALL_CLI[@]}" "$@" --session "$s"
}

# hd_into <var> <placeholder> <hall session> <hall args...>
hd_into() {
  local __v=$1 __p=$2 s=$3
  shift 3
  check_session "$s"
  run_into "$__v" "$__p" "${HALL_CLI[@]}" "$@" --session "$s"
}

# The one hall call that names no session: the read-only list of hall's
# sessions, from hall's own folder. It never sees herdr's.
hall_session_list() {
  run_into "$1" '{"sessions":[]}' "${HALL_CLI[@]}" session list --json
}

# What a home that passes the root check but fails the stand-in's walk inside it
# does to this run: block (refuse it, as every write or launch does) or warn
# (`down` and `status`: go on, reading and writing nothing inside the home, so a
# home a test claude dirtied can still be brought down). check_home sets
# HOME_WALKED to 1 when the walk passed.
HOME_WALK=block
HOME_WALKED=0

# check_home <path>: refuses <path> unless it is a test reeve home, and sets
# HALL_DEV_HOME to it as resolved on disk. The stand-in is the one judge of that:
# first `hall-standin check-root`, which follows symlinks, ignores letter case,
# and rules out ~/.reeve for HOME and for the user database's home alike, and
# always blocks; then `hall-standin check-home`, which also walks inside the
# home, and blocks or warns as HOME_WALK says. Without the stand-in there is no
# weaker check to fall back on, so a missing stand-in is a refusal too.
check_home() {
  local h=$1 real
  [[ $h =~ ^[A-Za-z0-9_./-]+$ ]] || die "refusing reeve home $h: use letters, digits, '.', '_', '-' and '/' only"
  case "/$h/" in */../* | */./*) die "refusing reeve home $h: no . or .. in the path" ;; esac
  [ -x "$HALL_STANDIN" ] || die "refusing reeve home $h: the stand-in $HALL_STANDIN, which checks it, is not built; build it first: PATH=$HALL_RUSTUP_BIN:\$PATH cargo build --manifest-path $HALL_REPO/Cargo.toml -p hall-standin"
  real=$("$HALL_STANDIN" check-root --home "$h") || die "refusing reeve home $h, as the stand-in says above"
  # The resolved path goes into the pane command line, so it must be as plain as the given one.
  [[ $real =~ ^/[A-Za-z0-9_./-]+$ ]] || die "refusing reeve home $h: it resolves to $real; use letters, digits, '.', '_', '-' and '/' only"
  HALL_DEV_HOME=$real
  if "$HALL_STANDIN" check-home --home "$real" >/dev/null; then
    HOME_WALKED=1
  elif [ "$HOME_WALK" = warn ]; then
    printf '%s: WARNING: the test reeve home %s fails the check inside it, as the stand-in says above; going on without reading or writing anything inside it\n' "$HALL_PROG" "$real" >&2
  else
    die "refusing reeve home $h, as the stand-in says above"
  fi
}

# parse_common <args...>: takes the shared flags, leaves the rest in REST.
parse_common() {
  REST=()
  while [ $# -gt 0 ]; do
    case $1 in
      --dry-run) DRY_RUN=1 ;;
      --home) [ $# -ge 2 ] || usage_die "--home needs a path"; ARG_HOME=$2; shift ;;
      --session) [ $# -ge 2 ] || usage_die "--session needs a name"; ARG_SESSION=$2; shift ;;
      --listing) [ $# -ge 2 ] || usage_die "--listing needs standin or reeve"; ARG_LISTING=$2; shift ;;
      --config) [ $# -ge 2 ] || usage_die "--config needs a file"; CONFIG=$2; shift ;;
      -h | --help) say "$HALL_USAGE"; exit 0 ;;
      --*) usage_die "unknown option $1" ;;
      *) REST+=("$1") ;;
    esac
    shift
  done
}

# load_inputs: the config file, then the flags over it; then every input checked and printed.
load_inputs() {
  : "${HOME:?HOME must be set, to rule out the live reeve home}"
  HALL_DEV_HOME="" HALL_DEV_SESSION="" HALL_DEV_LISTING="" HALL_DEV_REEVE_LISTING=""
  HALL_DEV_ROSTER=()
  HALL_DEV_REEVE_ROSTER=()
  if [ -n "$CONFIG" ]; then
    [ -f "$CONFIG" ] || die "config file $CONFIG not found"
    # shellcheck source=test-setup.conf
    . "$CONFIG"
  fi
  [ -n "$ARG_HOME" ] && HALL_DEV_HOME=$ARG_HOME
  [ -n "$ARG_SESSION" ] && HALL_DEV_SESSION=$ARG_SESSION
  [ -n "$ARG_LISTING" ] && HALL_DEV_LISTING=$ARG_LISTING
  case $HALL_DEV_HOME in "~/"*) HALL_DEV_HOME="$HOME/${HALL_DEV_HOME#"~/"}" ;; esac
  case $HALL_DEV_REEVE_LISTING in "~/"*) HALL_DEV_REEVE_LISTING="$HOME/${HALL_DEV_REEVE_LISTING#"~/"}" ;; esac

  [ -n "$HALL_DEV_HOME" ] || usage_die "no reeve home: pass --home or set HALL_DEV_HOME in the config"
  [ -n "$HALL_DEV_SESSION" ] || usage_die "no hall session: pass --session or set HALL_DEV_SESSION"
  check_home "$HALL_DEV_HOME"
  check_session "$HALL_DEV_SESSION"
  [ "${#HALL_DEV_ROSTER[@]}" -gt 0 ] || die "no roster: set HALL_DEV_ROSTER in the config"
  local line
  for line in "${HALL_DEV_ROSTER[@]}"; do roster_read "$line"; done
  check_listing

  say "test setup inputs:"
  say "  reeve home:      $HALL_DEV_HOME"
  say "  hall session:    $HALL_DEV_SESSION"
  say "  hall:            $HALL_BIN"
  say "  config:          ${CONFIG:-none}"
  if [ "$HALL_DEV_LISTING" = reeve ]; then
    say "  listing:         reeve's, $HALL_DEV_REEVE_LISTING --home $HALL_DEV_HOME (one test reeve, no hands placed)"
  else
    say "  listing:         the stand-in's, from the world file"
  fi
  [ "$HOME_WALKED" = 1 ] || say "  WARNING:         the test reeve home fails the check inside it (see stderr); nothing inside it is read or written"
  [ "$DRY_RUN" = 1 ] && say "  dry run:         printing commands, running none"
  return 0
}

# check_listing: which listing the reeve tree reads (HALL_DEV_LISTING or
# --listing). `standin`: the stand-in's, from the world file, with the whole
# roster placed. `reeve`: reeve's own listing command (HALL_DEV_REEVE_LISTING)
# on the test home, and the roster is HALL_DEV_REEVE_ROSTER, one test reeve and
# no test hand: the test reeve dispatches its hands itself, through reeve's hall
# backend (plan step 5, decision 68). The command goes into hall.toml as it is,
# so it must be a plain absolute path. Sets LISTING_CMD to the command.
check_listing() {
  case $HALL_DEV_LISTING in
    standin) LISTING_CMD=("$HALL_STANDIN" --home "$HALL_DEV_HOME") ;;
    reeve)
      [[ $HALL_DEV_REEVE_LISTING =~ ^/[A-Za-z0-9_./-]+$ ]] || die "refusing reeve listing command $(q "$HALL_DEV_REEVE_LISTING"): set HALL_DEV_REEVE_LISTING to an absolute path of letters, digits, '.', '_', '-' and '/' only"
      [ "${#HALL_DEV_REEVE_ROSTER[@]}" -eq 1 ] || die "the reeve listing starts one test reeve: set HALL_DEV_REEVE_ROSTER to one 'reeve <name>' line"
      roster_read "${HALL_DEV_REEVE_ROSTER[0]}"
      [ "$R_KIND" = reeve ] || die "the reeve listing places no test hand: HALL_DEV_REEVE_ROSTER is one 'reeve <name>' line"
      HALL_DEV_ROSTER=("${HALL_DEV_REEVE_ROSTER[@]}")
      LISTING_CMD=("$HALL_DEV_REEVE_LISTING" --home "$HALL_DEV_HOME")
      ;;
    *) usage_die "unknown listing ${HALL_DEV_LISTING:-(none)}: set HALL_DEV_LISTING or pass --listing, standin or reeve" ;;
  esac
}

# roster_find <name>: sets R_KIND R_NAME R_SESSION R_OWNER R_OFFICE for that line.
roster_find() {
  local line
  for line in "${HALL_DEV_ROSTER[@]}"; do
    roster_read "$line"
    [ "$R_NAME" = "$1" ] && return 0
  done
  return 1
}

# roster_read <line>: `reeve <name>` or `errand <id> <owner reeve> <office>`.
# Every line sits in the one hall session (decision 38), a test hand as a tab in
# its owner's workspace.
roster_read() {
  local extra
  read -r R_KIND R_NAME R_OWNER R_OFFICE extra <<<"$1"
  R_SESSION=$HALL_DEV_SESSION
  case $R_KIND in
    reeve) [ -z "$R_OWNER" ] || die "roster line '$1': a reeve line is 'reeve <name>' (no session number since step 2)" ;;
    errand) [ -n "$R_OFFICE" ] && [ -z "$extra" ] || die "roster line '$1': an errand line is 'errand <id> <owner reeve> <office>' (no session number since step 2)" ;;
    *) die "roster line '$1': kind is reeve or errand" ;;
  esac
  [[ $R_NAME =~ ^[a-z][a-z0-9-]{0,31}$ ]] || die "roster name $R_NAME: use a-z, 0-9 and '-', starting with a letter"
}

# check_pane_id <id> <what>: the stand-in's pane id rule (is_pane_id), checked
# before the id reaches hall's command line, where a leading '-' would read as
# an option.
check_pane_id() {
  [[ $1 =~ ^[A-Za-z0-9:_][A-Za-z0-9:_-]*$ ]] || die "refusing pane id $(q "$1") from $2: not letters, digits, ':', '_' and '-', starting with no '-'"
}

work_dir() { printf '%s/work/%s\n' "$HALL_DEV_HOME" "$1"; }

# The throwaway config and state folders every test claude runs with (decisions
# 47 and 48).
xdg_dir() { printf '%s/xdg-config\n' "$HALL_DEV_HOME"; }
xdg_state_dir() { printf '%s/xdg-state\n' "$HALL_DEV_HOME"; }

# claude_line <kind> <name>: the command line a test reeve or test hand runs in
# its pane (decisions 26, 47 and 48). It drops CLAUDE_DROP's names, whoever
# started the hall server; a test hand then gets its REEVE_HAND back, set on
# purpose to its errand id. HOME is never changed: that would move ~/.claude.
claude_line() {
  local hand=""
  [ "$1" = errand ] && hand="REEVE_HAND=\"$2\" "
  printf 'cd "%s" && env %s %sREEVE_HOME="%s" XDG_CONFIG_HOME="%s" XDG_STATE_HOME="%s" HALL_SOCKET_PATH="%s" claude --model %s' \
    "$(work_dir "$2")" "$CLAUDE_DROP" "$hand" "$HALL_DEV_HOME" "$(xdg_dir)" "$(xdg_state_dir)" "$HALL_DEV_SOCKET" "$HALL_MODEL"
}

standin() { run "$HALL_STANDIN" --home "$HALL_DEV_HOME" "$@"; }

# trust_report <dir>: whether claude's folder trust covers <dir>. Read only: the
# trust is the liege's to grant, in claude's own dialog, never by this script.
trust_report() {
  local dir=$1 d found=""
  say "+ $(q jq -r --arg d "$dir" '.projects[$d].hasTrustDialogAccepted // empty' "$HOME/.claude.json") (and each parent)"
  if [ "$DRY_RUN" = 1 ]; then
    return 0
  fi
  if [ ! -r "$HOME/.claude.json" ]; then
    say "  folder trust for $dir: unknown, ~/.claude.json not readable"
    return 0
  fi
  d=$dir
  while :; do
    if [ "$(jq -r --arg d "$d" '.projects[$d].hasTrustDialogAccepted // empty' "$HOME/.claude.json")" = true ]; then
      found=$d
      break
    fi
    [ "$d" = / ] && break
    d=$(dirname "$d")
  done
  if [ -n "$found" ]; then
    say "  folder trust for $dir: accepted (recorded for $found)"
  else
    say "  folder trust for $dir: NOT accepted; claude will open its trust dialog, which is the liege's to answer (hall reads it as blocked)"
  fi
}

# explain_report <hall session> <pane>: hall's reading of that agent, from its screen.
explain_report() {
  local s=$1 pane=$2 doc state box
  hd_into doc '{}' "$s" agent explain "$pane" --json || {
    say "  $pane in $s: agent explain failed"
    return 0
  }
  [ "$DRY_RUN" = 1 ] && return 0
  state=$(printf '%s' "$doc" | jq -r '.state // "unknown"')
  box=$(printf '%s' "$doc" | jq -r '[.evaluated_rules[]? | select(.id == "live_prompt_box") | .matched] | first // false')
  local reading
  case $state in
    blocked) reading="at a dialog (hall: blocked)" ;;
    working) reading="working" ;;
    idle | done)
      if [ "$box" = true ]; then reading="ready for input (turn ended)"; else reading="covered composer: a dialog is likely on screen"; fi
      ;;
    *) reading="unknown" ;;
  esac
  say "  $pane in $s: state=$state live_prompt_box=$box, reads as $reading"
  EXPLAIN_STATE=$state
  EXPLAIN_BOX=$box
}
