use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let env_home = std::env::var_os("HOME").map(PathBuf::from);
    let user_homes = hall_standin::home::live_user_homes(env_home.as_deref());
    match hall_standin::cli::run(&args, user_homes, &mut std::io::stdout()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("hall-standin: {}", e.message());
            ExitCode::from(e.exit_code() as u8)
        }
    }
}
