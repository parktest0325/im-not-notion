use anyhow::Result;
use ssh2::Channel;
use std::io::prelude::*;

pub fn run_command(channel: &mut Channel, command: &str) -> Result<String> {
    channel.exec(command)?;

    let mut s = String::new();
    channel.stderr().read_to_string(&mut s)?;
    println!("run_command stderr: {}", s);
    if s.is_empty() {
        channel.read_to_string(&mut s)?;
        println!("run_command stdout: {}", s);
    }
    Ok(s)
}
