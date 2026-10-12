# shellcheck shell=bash
# Shared by scripts/test-setup and scripts/test-drive. Sourced, never run.
#
# The rules every function here keeps (ADR 0003, decisions 24 and 26):
# - every herdr call names its herdr session with --session on the command line
#   and runs under `env -u HERDR_SOCKET_PATH -u HERDR_SESSION`, because a process
#   inside a live herdr pane inherits a HERDR_SOCKET_PATH pointing at default;
# - the herdr session default and the reeve home ~/.reeve are refused, the home
#   by the stand-in's check-home before anything else happens;
# - every test reeve and test hand claude starts with REEVE_HOME=<test home> on
#   its own command line;
# - --dry-run prints every command and runs none.

HALL_REPO=$(cd -P "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
HALL_MODEL=claude-haiku-4-5-20251001
HALL_RUSTUP_BIN=/opt/homebrew/opt/rustup/bin
# A prebuilt stand-in may be named here (the tests do); otherwise up builds it.
HALL_STANDIN="${HALL_STANDIN_BIN:-$HALL_REPO/target/debug/hall-standin}"
HALL_HERDR=(env -u HERDR_SOCKET_PATH -u HERDR_SESSION herdr)

DRY_RUN=0
CONFIG="$HALL_REPO/scripts/test-setup.conf"
ARG_HOME=""
ARG_SESSION_1=""
ARG_SESSION_2=""

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

# write_file <path> <text>: print what is written, then write it unless this is a dry run.
write_file() {
  say "+ write $(q "$1")"
  [ "$DRY_RUN" = 1 ] && return 0
  printf '%s' "$2" >"$1"
}

check_session() {
  local s=$1
  shopt -s nocasematch
  if [[ $s == default ]]; then
    shopt -u nocasematch
    die "refusing herdr session $s: it belongs to the live setup"
  fi
  shopt -u nocasematch
  [[ $s =~ ^hall-dev[a-z0-9-]*$ ]] || die "refusing herdr session $s: test herdr sessions are named hall-dev*"
}

# hd <herdr session> <herdr args...>: one herdr call against that session.
hd() {
  local s=$1
  shift
  check_session "$s"
  run "${HALL_HERDR[@]}" "$@" --session "$s"
}

# hd_into <var> <placeholder> <herdr session> <herdr args...>
hd_into() {
  local __v=$1 __p=$2 s=$3
  shift 3
  check_session "$s"
  run_into "$__v" "$__p" "${HALL_HERDR[@]}" "$@" --session "$s"
}

# The one herdr call that names no session: the read-only list of all of them.
herdr_session_list() {
  run_into "$1" '{"sessions":[]}' "${HALL_HERDR[@]}" session list --json
}

# check_home <path>: refuses <path> unless it is a test reeve home, and sets
# HALL_DEV_HOME to it as resolved on disk. The stand-in is the one judge of that
# (`hall-standin check-home`): it follows symlinks, ignores letter case, and rules
# out ~/.reeve for HOME and for the user database's home alike. Without it there
# is no weaker check to fall back on, so a missing stand-in is a refusal too.
check_home() {
  local h=$1 real
  [[ $h =~ ^[A-Za-z0-9_./-]+$ ]] || die "refusing reeve home $h: use letters, digits, '.', '_', '-' and '/' only"
  case "/$h/" in */../* | */./*) die "refusing reeve home $h: no . or .. in the path" ;; esac
  [ -x "$HALL_STANDIN" ] || die "refusing reeve home $h: the stand-in $HALL_STANDIN, which checks it, is not built; build it first: PATH=$HALL_RUSTUP_BIN:\$PATH cargo build --manifest-path $HALL_REPO/Cargo.toml -p hall-standin"
  real=$("$HALL_STANDIN" check-home --home "$h") || die "refusing reeve home $h, as the stand-in says above"
  # The resolved path goes into the pane command line, so it must be as plain as the given one.
  [[ $real =~ ^/[A-Za-z0-9_./-]+$ ]] || die "refusing reeve home $h: it resolves to $real; use letters, digits, '.', '_', '-' and '/' only"
  HALL_DEV_HOME=$real
}

# parse_common <args...>: takes the shared flags, leaves the rest in REST.
parse_common() {
  REST=()
  while [ $# -gt 0 ]; do
    case $1 in
      --dry-run) DRY_RUN=1 ;;
      --home) [ $# -ge 2 ] || usage_die "--home needs a path"; ARG_HOME=$2; shift ;;
      --session-1) [ $# -ge 2 ] || usage_die "--session-1 needs a name"; ARG_SESSION_1=$2; shift ;;
      --session-2) [ $# -ge 2 ] || usage_die "--session-2 needs a name"; ARG_SESSION_2=$2; shift ;;
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
  HALL_DEV_HOME="" HALL_DEV_SESSION_1="" HALL_DEV_SESSION_2=""
  HALL_DEV_ROSTER=()
  if [ -n "$CONFIG" ]; then
    [ -f "$CONFIG" ] || die "config file $CONFIG not found"
    # shellcheck source=test-setup.conf
    . "$CONFIG"
  fi
  [ -n "$ARG_HOME" ] && HALL_DEV_HOME=$ARG_HOME
  [ -n "$ARG_SESSION_1" ] && HALL_DEV_SESSION_1=$ARG_SESSION_1
  [ -n "$ARG_SESSION_2" ] && HALL_DEV_SESSION_2=$ARG_SESSION_2
  case $HALL_DEV_HOME in "~/"*) HALL_DEV_HOME="$HOME/${HALL_DEV_HOME#"~/"}" ;; esac

  [ -n "$HALL_DEV_HOME" ] || usage_die "no reeve home: pass --home or set HALL_DEV_HOME in the config"
  [ -n "$HALL_DEV_SESSION_1" ] || usage_die "no first herdr session: pass --session-1 or set HALL_DEV_SESSION_1"
  [ -n "$HALL_DEV_SESSION_2" ] || usage_die "no second herdr session: pass --session-2 or set HALL_DEV_SESSION_2"
  check_home "$HALL_DEV_HOME"
  check_session "$HALL_DEV_SESSION_1"
  check_session "$HALL_DEV_SESSION_2"
  [ "$HALL_DEV_SESSION_1" != "$HALL_DEV_SESSION_2" ] || die "the two herdr sessions must differ"
  [ "${#HALL_DEV_ROSTER[@]}" -gt 0 ] || die "no roster: set HALL_DEV_ROSTER in the config"

  say "test setup inputs:"
  say "  reeve home:      $HALL_DEV_HOME"
  say "  herdr sessions:  $HALL_DEV_SESSION_1 $HALL_DEV_SESSION_2"
  say "  config:          ${CONFIG:-none}"
  [ "$DRY_RUN" = 1 ] && say "  dry run:         printing commands, running none"
  return 0
}

# roster_session <1|2>: the herdr session name for a roster session number.
roster_session() {
  case $1 in
    1) printf '%s\n' "$HALL_DEV_SESSION_1" ;;
    2) printf '%s\n' "$HALL_DEV_SESSION_2" ;;
    *) die "roster names herdr session $1; use 1 or 2" ;;
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

roster_read() {
  local n
  read -r R_KIND R_NAME n R_OWNER R_OFFICE <<<"$1"
  R_SESSION=$(roster_session "$n")
  [[ $R_NAME =~ ^[a-z][a-z0-9-]{0,31}$ ]] || die "roster name $R_NAME: use a-z, 0-9 and '-', starting with a letter"
}

work_dir() { printf '%s/work/%s\n' "$HALL_DEV_HOME" "$1"; }

# The command line a test reeve or test hand runs in its pane (decision 26).
claude_line() {
  printf 'cd "%s" && REEVE_HOME="%s" claude --model %s' "$(work_dir "$1")" "$HALL_DEV_HOME" "$HALL_MODEL"
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
    say "  folder trust for $dir: NOT accepted; claude will open its trust dialog, which is the liege's to answer (herdr reads it as blocked)"
  fi
}

# explain_report <herdr session> <pane>: herdr's reading of that agent, from the live screen.
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
    blocked) reading="at a dialog (herdr: blocked)" ;;
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
