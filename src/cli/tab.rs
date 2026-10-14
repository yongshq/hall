use std::collections::HashMap;

use crate::api::schema::{TabCreateParams, TabListParams, TabRenameParams};

pub(super) fn run_tab_command(args: &[String]) -> std::io::Result<i32> {
    let Some(subcommand) = args.first().map(|arg| arg.as_str()) else {
        print_tab_help();
        return Ok(2);
    };

    match subcommand {
        "list" => tab_list(&args[1..]),
        "create" => tab_create(&args[1..]),
        "get" => tab_get(&args[1..]),
        "focus" => tab_focus(&args[1..]),
        "rename" => tab_rename(&args[1..]),
        "close" => tab_close(&args[1..]),
        "help" | "--help" | "-h" => {
            print_tab_help();
            Ok(0)
        }
        _ => {
            print_tab_help();
            Ok(2)
        }
    }
}

fn tab_list(args: &[String]) -> std::io::Result<i32> {
    let mut workspace_id = None;

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
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }

    super::runtime::tab_list(TabListParams { workspace_id })
}

fn tab_create(args: &[String]) -> std::io::Result<i32> {
    let params = match parse_tab_create_args(args) {
        Ok(params) => params,
        Err(message) => {
            eprintln!("{message}");
            return Ok(2);
        }
    };

    super::runtime::tab_create(params)
}

fn parse_tab_create_args(args: &[String]) -> Result<TabCreateParams, String> {
    let mut workspace_id = None;
    let mut cwd = None;
    let mut focus = false;
    let mut label = None;
    let mut env = HashMap::new();
    let mut no_agent_resume = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--workspace" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("missing value for --workspace".into());
                };
                workspace_id = Some(super::normalize_workspace_id(value));
                index += 2;
            }
            "--cwd" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("missing value for --cwd".into());
                };
                cwd = Some(value.clone());
                index += 2;
            }
            "--label" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("missing value for --label".into());
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
            "--no-agent-resume" => {
                no_agent_resume = true;
                index += 1;
            }
            "--env" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("missing value for --env".into());
                };
                let (key, value) = super::parse_env_assignment(value)?;
                env.insert(key, value);
                index += 2;
            }
            other => return Err(format!("unknown option: {other}")),
        }
    }

    Ok(TabCreateParams {
        workspace_id,
        cwd,
        focus,
        label,
        env,
        no_agent_resume,
    })
}

fn tab_get(args: &[String]) -> std::io::Result<i32> {
    let Some(raw_tab_id) = args.first() else {
        eprintln!("usage: hall tab get <tab_id>");
        return Ok(2);
    };
    if args.len() != 1 {
        eprintln!("usage: hall tab get <tab_id>");
        return Ok(2);
    }

    super::runtime::tab_get(super::normalize_tab_id(raw_tab_id))
}

fn tab_focus(args: &[String]) -> std::io::Result<i32> {
    let Some(raw_tab_id) = args.first() else {
        eprintln!("usage: hall tab focus <tab_id>");
        return Ok(2);
    };
    if args.len() != 1 {
        eprintln!("usage: hall tab focus <tab_id>");
        return Ok(2);
    }

    super::runtime::tab_focus(super::normalize_tab_id(raw_tab_id))
}

fn tab_rename(args: &[String]) -> std::io::Result<i32> {
    if args.len() < 2 {
        eprintln!("usage: hall tab rename <tab_id> <label>");
        return Ok(2);
    }

    super::runtime::tab_rename(TabRenameParams {
        tab_id: super::normalize_tab_id(&args[0]),
        label: args[1..].join(" "),
    })
}

fn tab_close(args: &[String]) -> std::io::Result<i32> {
    let Some(raw_tab_id) = args.first() else {
        eprintln!("usage: hall tab close <tab_id>");
        return Ok(2);
    };
    if args.len() != 1 {
        eprintln!("usage: hall tab close <tab_id>");
        return Ok(2);
    }

    super::runtime::tab_close(super::normalize_tab_id(raw_tab_id))
}

fn print_tab_help() {
    eprintln!("hall tab commands:");
    eprintln!("  hall tab list [--workspace <workspace_id>]");
    eprintln!(
        "  hall tab create [--workspace <workspace_id>] [--cwd PATH] [--label TEXT] [--env KEY=VALUE] [--focus] [--no-focus] [--no-agent-resume]"
    );
    eprintln!("  hall tab get <tab_id>");
    eprintln!("  hall tab focus <tab_id>");
    eprintln!("  hall tab rename <tab_id> <label>");
    eprintln!("  hall tab close <tab_id>");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn tab_create_parses_no_agent_resume() {
        let marked =
            parse_tab_create_args(&args(&["--workspace", "w1", "--no-agent-resume"])).unwrap();
        assert!(marked.no_agent_resume);
        assert_eq!(
            serde_json::to_value(&marked).unwrap()["no_agent_resume"],
            serde_json::json!(true)
        );

        let unmarked = parse_tab_create_args(&args(&["--workspace", "w1"])).unwrap();
        assert!(!unmarked.no_agent_resume);
        assert!(serde_json::to_value(&unmarked)
            .unwrap()
            .get("no_agent_resume")
            .is_none());
    }

    #[test]
    fn tab_create_reports_usage_errors() {
        assert_eq!(
            parse_tab_create_args(&args(&["--no-agent-resume", "--bogus"])).unwrap_err(),
            "unknown option: --bogus"
        );
        assert_eq!(
            parse_tab_create_args(&args(&["--env", "KEY"])).unwrap_err(),
            "env must use KEY=VALUE"
        );
    }
}
