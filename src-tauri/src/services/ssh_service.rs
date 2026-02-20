use ssh2::{Session, Channel, Sftp};
use std::{net::{TcpStream, ToSocketAddrs}, sync::Mutex, io::Read, time::Duration};
use anyhow::{Result, Context};
use crate::types::config::SshConfig;
use once_cell::sync::Lazy;

const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

static SSH_CLIENT: Lazy<Mutex<Option<Session>>> = Lazy::new(|| Mutex::new(None));

/// SshConfig를 직접 받아 SSH 연결
fn connect_inner(ssh_config: &SshConfig, force: bool) -> Result<()> {
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
    let addr = format!("{}:{}", ssh_config.host, ssh_config.port);
    let sock_addr = addr.to_socket_addrs()
        .context("Failed to resolve SSH address")?
        .next()
        .context("No address found for SSH host")?;
    let tcp = TcpStream::connect_timeout(&sock_addr, TCP_CONNECT_TIMEOUT)
        .context("Failed to connect to SSH server (timeout)")?;
    session.set_tcp_stream(tcp);
    session.handshake().context("Failed to perform SSH handshake")?;

    if !ssh_config.password.is_empty() {
        session.userauth_password(&ssh_config.username, &ssh_config.password)
            .context("Failed to authenticate with password")?;
    }

    let mut ssh_client = SSH_CLIENT.lock().unwrap();
    *ssh_client = Some(session);

    Ok(())
}

/// SshConfig를 직접 받아 연결 (기존 세션 재사용)
pub fn connect_ssh_with_config(ssh_config: &SshConfig) -> Result<()> {
    connect_inner(ssh_config, false)
}

/// SshConfig를 직접 받아 강제 재연결
pub fn reconnect_ssh_with_config(ssh_config: &SshConfig) -> Result<()> {
    connect_inner(ssh_config, true)
}


/// SSH 세션이 살아있는지 확인
pub fn is_ssh_connected() -> bool {
    let client = SSH_CLIENT.lock().unwrap();
    if let Some(ref session) = *client {
        session.authenticated() && session.channel_session().is_ok()
    } else {
        false
    }
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

/// SSH 서버의 홈 디렉토리 경로를 가져옴
pub fn get_server_home_path() -> Result<String> {
    let mut channel = get_channel_session()?;
    let output = execute_ssh_command(&mut channel, "echo $HOME")?;
    Ok(output.trim().to_string())
}

pub fn execute_ssh_command(channel: &mut Channel, command: &str) -> Result<String> {
    channel.exec(command).context("Failed to execute SSH command")?;

    let mut stdout = String::new();
    channel.read_to_string(&mut stdout).context("Failed to read from SSH stdout")?;

    let mut stderr = String::new();
    channel.stderr().read_to_string(&mut stderr).context("Failed to read from SSH stderr")?;

    if !stderr.is_empty() {
        eprintln!("run_command stderr: {}", stderr);
    }

    Ok(stdout)
}
