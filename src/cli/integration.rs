use crate::api::schema::IntegrationTarget;

pub(super) fn run_integration_command(args: &[String]) -> std::io::Result<i32> {
    let Some(subcommand) = args.first().map(|arg| arg.as_str()) else {
        print_integration_help();
        return Ok(2);
    };

    match subcommand {
        // hall: refused; herdr's hooks under ~/.claude and the rest belong to the live herdr.
        "install" | "uninstall" if crate::integration::HALL_REFUSES_INSTALL => {
            eprintln!("hall integration {subcommand} is disabled: it would write herdr's agent hooks (e.g. ~/.claude/hooks/herdr-agent-state.sh), which belong to the installed herdr");
            Ok(1)
        }
        "install" => integration_install(&args[1..]),
        "uninstall" => integration_uninstall(&args[1..]),
        "status" => integration_status(&args[1..]),
        "help" | "--help" | "-h" => {
            print_integration_help();
            Ok(0)
        }
        _ => {
            print_integration_help();
            Ok(2)
        }
    }
}

fn integration_status(args: &[String]) -> std::io::Result<i32> {
    let outdated_only = match args {
        [] => false,
        [flag] if flag == "--outdated-only" => true,
        _ => {
            eprintln!("usage: hall integration status [--outdated-only]");
            return Ok(2);
        }
    };

    if outdated_only {
        crate::integration::print_outdated_update_notice();
        return Ok(0);
    }

    for status in crate::integration::installed_integration_statuses() {
        let target = crate::integration::integration_target_label(status.target);
        let version = match status.installed_version {
            Some(version) => format!("v{version}"),
            None => "legacy".to_string(),
        };
        let state = match status.state {
            crate::integration::IntegrationStatusKind::NotInstalled => "not installed".to_string(),
            crate::integration::IntegrationStatusKind::Current => {
                format!("current ({version})")
            }
            crate::integration::IntegrationStatusKind::Outdated
                if status
                    .installed_version
                    .is_some_and(|installed| installed >= status.expected_version) =>
            {
                format!("needs repair ({version})")
            }
            crate::integration::IntegrationStatusKind::Outdated => {
                format!("outdated ({version} < v{})", status.expected_version)
            }
        };
        println!("{target}: {state} ({})", status.path.display());
    }

    Ok(0)
}

fn integration_install(args: &[String]) -> std::io::Result<i32> {
    let Some(target) = parse_integration_target(args, "install")? else {
        return Ok(2);
    };

    match crate::integration::install_target(target) {
        Ok(messages) => {
            print_integration_messages(messages);
            Ok(0)
        }
        Err(err) => {
            eprintln!("{err}");
            Ok(1)
        }
    }
}

fn integration_uninstall(args: &[String]) -> std::io::Result<i32> {
    let Some(target) = parse_integration_target(args, "uninstall")? else {
        return Ok(2);
    };

    match crate::integration::uninstall_target(target) {
        Ok(messages) => {
            print_integration_messages(messages);
            Ok(0)
        }
        Err(err) => {
            eprintln!("{err}");
            Ok(1)
        }
    }
}

fn print_integration_messages(messages: Vec<String>) {
    for message in messages {
        println!("{message}");
    }
}

fn parse_integration_target(
    args: &[String],
    action: &str,
) -> std::io::Result<Option<IntegrationTarget>> {
    let Some(target) = args.first().map(|arg| arg.as_str()) else {
        eprintln!(
            "usage: hall integration {action} <pi|omp|claude|codex|copilot|devin|droid|kimi|opencode|kilo|hermes|qodercli|qwen|cursor|mastracode|grok>"
        );
        return Ok(None);
    };
    if args.len() != 1 {
        eprintln!(
            "usage: hall integration {action} <pi|omp|claude|codex|copilot|devin|droid|kimi|opencode|kilo|hermes|qodercli|qwen|cursor|mastracode|grok>"
        );
        return Ok(None);
    }

    let parsed = match target {
        "pi" => IntegrationTarget::Pi,
        "omp" => IntegrationTarget::Omp,
        "claude" => IntegrationTarget::Claude,
        "codex" => IntegrationTarget::Codex,
        "copilot" => IntegrationTarget::Copilot,
        "devin" => IntegrationTarget::Devin,
        "droid" => IntegrationTarget::Droid,
        "kimi" => IntegrationTarget::Kimi,
        "opencode" => IntegrationTarget::Opencode,
        "kilo" => IntegrationTarget::Kilo,
        "hermes" => IntegrationTarget::Hermes,
        "qodercli" => IntegrationTarget::Qodercli,
        "qwen" => IntegrationTarget::Qwen,
        "cursor" => IntegrationTarget::Cursor,
        "mastracode" => IntegrationTarget::Mastracode,
        "antigravity-cli" | "antigravity_cli" => IntegrationTarget::AntigravityCli,
        "grok" => IntegrationTarget::Grok,
        _ => {
            eprintln!("unknown integration target: {target}");
            eprintln!(
                "currently supported: pi, omp, claude, codex, copilot, devin, droid, kimi, opencode, kilo, hermes, qodercli, qwen, cursor, mastracode, antigravity-cli, grok"
            );
            return Ok(None);
        }
    };

    Ok(Some(parsed))
}

fn print_integration_help() {
    eprintln!("hall integration commands:");
    eprintln!("  hall integration install pi");
    eprintln!("  hall integration install omp");
    eprintln!("  hall integration install claude");
    eprintln!("  hall integration install codex");
    eprintln!("  hall integration install copilot");
    eprintln!("  hall integration install devin");
    eprintln!("  hall integration install droid");
    eprintln!("  hall integration install kimi");
    eprintln!("  hall integration install opencode");
    eprintln!("  hall integration install kilo");
    eprintln!("  hall integration install hermes");
    eprintln!("  hall integration install qodercli");
    eprintln!("  hall integration install qwen");
    eprintln!("  hall integration install cursor");
    eprintln!("  hall integration install mastracode");
    eprintln!("  hall integration install antigravity-cli");
    eprintln!("  hall integration install grok");
    eprintln!("  hall integration uninstall pi");
    eprintln!("  hall integration uninstall omp");
    eprintln!("  hall integration uninstall claude");
    eprintln!("  hall integration uninstall codex");
    eprintln!("  hall integration uninstall copilot");
    eprintln!("  hall integration uninstall devin");
    eprintln!("  hall integration uninstall droid");
    eprintln!("  hall integration uninstall kimi");
    eprintln!("  hall integration uninstall opencode");
    eprintln!("  hall integration uninstall kilo");
    eprintln!("  hall integration uninstall hermes");
    eprintln!("  hall integration uninstall qodercli");
    eprintln!("  hall integration uninstall qwen");
    eprintln!("  hall integration uninstall cursor");
    eprintln!("  hall integration uninstall mastracode");
    eprintln!("  hall integration uninstall antigravity-cli");
    eprintln!("  hall integration uninstall grok");
    eprintln!("  hall integration status [--outdated-only]");
}
