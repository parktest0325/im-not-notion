use anyhow::Result;
use ssh2::Channel;

use super::terminal::run_command;

pub fn hugo_new_content(
    channel: &mut Channel,
    base: &str,
    hugo_cmd_path: &str,
    path: &str,
) -> Result<()> {
    run_command(
        channel,
        &format!("cd {} ; {} new content {}", base, hugo_cmd_path, path),
    )?;
    Ok(())
}
