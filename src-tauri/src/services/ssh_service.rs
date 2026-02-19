use ssh2::{Session, Channel, Sftp};
use std::{net::TcpStream, path::Path, sync::Mutex, io::Read};
use anyhow::{Result, Context};
use crate::types::config::AppConfig;
use once_cell::sync::Lazy;

static SSH_CLIENT: Lazy<Mutex<Option<Session>>> = Lazy::new(|| Mutex::new(None));

/// Connect SSH. If `force` is false, reuses existing live session.
pub fn connect_ssh_inner(config: &AppConfig, force: bool) -> Result<()> {
    if !force {
        let client = SSH_CLIENT.lock().unwrap();
        if let Some(ref session) = *client {
            if session.authenticated() {
                // Try opening a channel to verify connection is alive
                if session.channel_session().is_ok() {
                    return Ok(());
                }
            }
        }
    }

    let mut session = Session::new().context("Failed to create SSH session")?;
    let tcp = TcpStream::connect(format!("{}:{}", config.ssh_config.host, config.ssh_config.port))
        .context("Failed to connect to SSH server")?;
    session.set_tcp_stream(tcp);
    session.handshake().context("Failed to perform SSH handshake")?;

    if !config.ssh_config.password.is_empty() {
        session.userauth_password(&config.ssh_config.username, &config.ssh_config.password)
            .context("Failed to authenticate with password")?;
    }
    // else {
    //     session.userauth_pubkey_file(
    //         &config.ssh_config.username,
    //         None,
    //         Path::new(&config.ssh_config.key_path),
    //         None,
    //     ).context("Failed to authenticate with public key")?;
    // }

    let mut ssh_client = SSH_CLIENT.lock().unwrap();
    *ssh_client = Some(session);

    Ok(())
}

/// Connect SSH, reusing existing session if alive
pub fn connect_ssh(config: &AppConfig) -> Result<()> {
    connect_ssh_inner(config, false)
}

/// Force reconnect SSH (used when credentials may have changed)
pub fn reconnect_ssh(config: &AppConfig) -> Result<()> {
    connect_ssh_inner(config, true)
}

pub fn get_channel_session() -> Result<Channel> {
    let channel = SSH_CLIENT.lock().unwrap()
        .as_ref()
        .context("SSH session not initialized")?
        .channel_session().context("Failed to open SSH channel session")?;
    Ok(channel)
}

pub fn get_sftp_session() -> Result<Sftp> {
    let sftp = SSH_CLIENT.lock().unwrap()
        .as_ref()
        .context("SSH session not initialized")?
        .sftp().context("Failed to open SFTP session")?;
    Ok(sftp)
}

pub fn execute_ssh_command(channel: &mut Channel, command: &str) -> Result<String> {
    match channel.exec(command) {
        Ok(_) => println!("Command executed successfully"),
        Err(e) => eprintln!("Failed to execute SSH command: {:#}", e),
    }

    let mut s = String::new();
    channel.stderr().read_to_string(&mut s).context("Failed to read from SSH stderr")?;
    println!("run_command stderr: {}", s);
    if s.is_empty() {
        channel.read_to_string(&mut s).context("Failed to read from SSH stdout")?;
        println!("run_command stdout: {}", s);
    }
    Ok(s)
}
