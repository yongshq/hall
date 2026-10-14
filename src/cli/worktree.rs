use crate::api::schema::{
    WorktreeCreateParams, WorktreeListParams, WorktreeOpenParams, WorktreeRemoveParams,
};

const WORKTREE_OPEN_USAGE: &str = "usage: hall worktree open [--workspace ID | --cwd PATH] (--path PATH | --branch NAME) [--label TEXT] [--focus] [--no-focus] [--trust-repository] [--no-agent-resume]";

// Worktree output is always JSON. The parsers retain `--json` as a hidden compatibility no-op.
pub(super) fn run_worktree_command(args: &[String]) -> std::io::Result<i32> {
    let Some(subcommand) = args.first().map(|arg| arg.as_str()) else {
        print_worktree_help();
        return Ok(2);
    };

    match subcommand {
        "list" => worktree_list(&args[1..]),
        "create" => worktree_create(&args[1..]),
        "open" => worktree_open(&args[1..]),
        "remove" => worktree_remove(&args[1..]),
        "help" | "--help" | "-h" => {
            print_worktree_help();
            Ok(0)
        }
        _ => {
            print_worktree_help();
            Ok(2)
        }
    }
}

fn worktree_list(args: &[String]) -> std::io::Result<i32> {
    let mut workspace_id = None;
    let mut cwd = None;
    let mut trust_repository = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--workspace" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --workspace");
                    return Ok(2);
                };
                workspace_id = Some(super::normalize_workspace_id(value));
                index += 2;
            }
            "--cwd" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --cwd");
                    return Ok(2);
                };
                cwd = Some(normalize_path_arg(value)?);
                index += 2;
            }
            "--trust-repository" => {
                trust_repository = true;
                index += 1;
            }
            "--json" => index += 1,
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }
    if workspace_id.is_some() && cwd.is_some() {
        eprintln!("usage: hall worktree list [--workspace ID | --cwd PATH] [--trust-repository]");
        return Ok(2);
    }

    super::runtime::worktree_list(WorktreeListParams {
        workspace_id,
        cwd,
        trust_repository,
    })
}

fn worktree_create(args: &[String]) -> std::io::Result<i32> {
    let mut workspace_id = None;
    let mut cwd = None;
    let mut branch = None;
    let mut base = None;
    let mut path = None;
    let mut label = None;
    let mut focus = false;
    let mut trust_repository = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--workspace" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --workspace");
                    return Ok(2);
                };
                workspace_id = Some(super::normalize_workspace_id(value));
                index += 2;
            }
            "--cwd" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --cwd");
                    return Ok(2);
                };
                cwd = Some(normalize_path_arg(value)?);
                index += 2;
            }
            "--branch" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --branch");
                    return Ok(2);
                };
                branch = Some(value.clone());
                index += 2;
            }
            "--base" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --base");
                    return Ok(2);
                };
                base = Some(value.clone());
                index += 2;
            }
            "--path" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --path");
                    return Ok(2);
                };
                path = Some(normalize_path_arg(value)?);
                index += 2;
            }
            "--label" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --label");
                    return Ok(2);
                };
                label = Some(value.clone());
                index += 2;
            }
            "--focus" => {
                focus = true;
                index += 1;
            }
            "--no-focus" => {
                focus = false;
                index += 1;
            }
            "--trust-repository" => {
                trust_repository = true;
                index += 1;
            }
            "--json" => index += 1,
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }
    if workspace_id.is_some() && cwd.is_some() {
        eprintln!(
            "usage: hall worktree create [--workspace ID | --cwd PATH] [--branch NAME] [--base REF] [--path PATH] [--label TEXT] [--focus] [--no-focus] [--trust-repository]"
        );
        return Ok(2);
    }

    super::runtime::worktree_create(WorktreeCreateParams {
        workspace_id,
        cwd,
        branch,
        base,
        path,
        label,
        focus,
        trust_repository,
    })
}

fn worktree_open(args: &[String]) -> std::io::Result<i32> {
    let params = match parse_worktree_open_args(args)? {
        Ok(params) => params,
        Err(message) => {
            eprintln!("{message}");
            return Ok(2);
        }
    };

    super::runtime::worktree_open(params)
}

/// The outer error is a failure to read the current directory for a relative
/// path; the inner one is a usage error.
fn parse_worktree_open_args(
    args: &[String],
) -> std::io::Result<Result<WorktreeOpenParams, String>> {
    let mut workspace_id = None;
    let mut cwd = None;
    let mut path = None;
    let mut branch = None;
    let mut label = None;
    let mut focus = false;
    let mut trust_repository = false;
    let mut no_agent_resume = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--workspace" => {
                let Some(value) = args.get(index + 1) else {
                    return Ok(Err("missing value for --workspace".into()));
                };
                workspace_id = Some(super::normalize_workspace_id(value));
                index += 2;
            }
            "--cwd" => {
                let Some(value) = args.get(index + 1) else {
                    return Ok(Err("missing value for --cwd".into()));
                };
                cwd = Some(normalize_path_arg(value)?);
                index += 2;
            }
            "--path" => {
                let Some(value) = args.get(index + 1) else {
                    return Ok(Err("missing value for --path".into()));
                };
                path = Some(normalize_path_arg(value)?);
                index += 2;
            }
            "--branch" => {
                let Some(value) = args.get(index + 1) else {
                    return Ok(Err("missing value for --branch".into()));
                };
                branch = Some(value.clone());
                index += 2;
            }
            "--label" => {
                let Some(value) = args.get(index + 1) else {
                    return Ok(Err("missing value for --label".into()));
                };
                label = Some(value.clone());
                index += 2;
            }
            "--focus" => {
                focus = true;
                index += 1;
            }
            "--no-focus" => {
                focus = false;
                index += 1;
            }
            "--trust-repository" => {
                trust_repository = true;
                index += 1;
            }
            "--no-agent-resume" => {
                no_agent_resume = true;
                index += 1;
            }
            "--json" => index += 1,
            other => return Ok(Err(format!("unknown option: {other}"))),
        }
    }
    if workspace_id.is_some() && cwd.is_some() {
        return Ok(Err(WORKTREE_OPEN_USAGE.into()));
    }
    if path.is_some() == branch.is_some() {
        return Ok(Err(WORKTREE_OPEN_USAGE.into()));
    }

    Ok(Ok(WorktreeOpenParams {
        workspace_id,
        cwd,
        path,
        branch,
        label,
        focus,
        trust_repository,
        no_agent_resume,
    }))
}

fn worktree_remove(args: &[String]) -> std::io::Result<i32> {
    let mut workspace_id = None;
    let mut force = false;
    let mut trust_repository = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--workspace" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --workspace");
                    return Ok(2);
                };
                workspace_id = Some(super::normalize_workspace_id(value));
                index += 2;
            }
            "--force" => {
                force = true;
                index += 1;
            }
            "--trust-repository" => {
                trust_repository = true;
                index += 1;
            }
            "--json" => index += 1,
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }

    let Some(workspace_id) = workspace_id else {
        eprintln!("usage: hall worktree remove --workspace ID [--force] [--trust-repository]");
        return Ok(2);
    };

    super::runtime::worktree_remove(WorktreeRemoveParams {
        workspace_id,
        force,
        trust_repository,
    })
}

fn print_worktree_help() {
    eprintln!("hall worktree commands:");
    eprintln!("  hall worktree list [--workspace ID | --cwd PATH] [--trust-repository]");
    eprintln!(
        "  hall worktree create [--workspace ID | --cwd PATH] [--branch NAME] [--base REF] [--path PATH] [--label TEXT] [--focus] [--no-focus] [--trust-repository]"
    );
    eprintln!(
        "  hall worktree open [--workspace ID | --cwd PATH] (--path PATH | --branch NAME) [--label TEXT] [--focus] [--no-focus] [--trust-repository] [--no-agent-resume]"
    );
    eprintln!("  hall worktree remove --workspace ID [--force] [--trust-repository]");
}

fn normalize_path_arg(value: &str) -> std::io::Result<String> {
    let path = crate::worktree::expand_tilde_path(value);
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(absolute.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn worktree_open_parses_no_agent_resume() {
        let marked = parse_worktree_open_args(&args(&["--path", "/tmp/wt", "--no-agent-resume"]))
            .unwrap()
            .unwrap();
        assert!(marked.no_agent_resume);
        assert_eq!(marked.path.as_deref(), Some("/tmp/wt"));

        let unmarked = parse_worktree_open_args(&args(&["--path", "/tmp/wt"]))
            .unwrap()
            .unwrap();
        assert!(!unmarked.no_agent_resume);
    }

    #[test]
    fn worktree_open_still_needs_path_or_branch() {
        assert_eq!(
            parse_worktree_open_args(&args(&["--no-agent-resume"]))
                .unwrap()
                .unwrap_err(),
            WORKTREE_OPEN_USAGE
        );
    }
}
