use std::process::ExitCode;

fn main() -> ExitCode {
    match telfhash_rs::run_cli() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}
