use std::process::ExitCode;

use walltz::Program;

#[tokio::main]
async fn main() -> ExitCode {
    Program::init().await
}
