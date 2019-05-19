use std::{
    env,
    error::Error,
    process::{self, Command},
};

fn main() -> Result<(), Box<Error>> {
    if let Some(path) = env::args().nth(1) {
        let mut c = Command::new("sudo");
        c.args(&["setcap", "cap_sys_nice+ep", &path]);
        eprintln!("$ {:?}", c);
        let status = c.status()?;

        if !status.success() {
            process::exit(status.code().unwrap_or(1))
        }

        eprintln!("$ {}", path);
        let status = Command::new(path).status()?;

        if !status.success() {
            process::exit(status.code().unwrap_or(1))
        }
    }

    Ok(())
}
