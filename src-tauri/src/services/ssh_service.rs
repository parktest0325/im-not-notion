use ssh2::{Session, Channel, Sftp};
use std::{net::{TcpStream, ToSocketAddrs}, sync::Mutex, io::Read, time::Duration, path::Path, ops::{Deref, DerefMut}};
use anyhow::{Result, Context};
use serde::Serialize;
use crate::types::config::SshConfig;
use crate::services::config_service::get_hugo_config;
use once_cell::sync::Lazy;

const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const ALIVE_CHECK_TIMEOUT_MS: u32 = 2000;

static SSH_CLIENT: Lazy<Mutex<Option<Session>>> = Lazy::new(|| Mutex::new(None));
static SFTP_CACHE: Lazy<Mutex<Option<Sftp>>> = Lazy::new(|| Mutex::new(None));

/// RAII wrapper: drop 시 SFTP 세션을 캐시에 반환
pub struct SftpHandle(Option<Sftp>);

impl Drop for SftpHandle {
    fn drop(&mut self) {
        if let Some(sftp) = self.0.take() {
            *SFTP_CACHE.lock().unwrap() = Some(sftp);
        }
    }
}

impl Deref for SftpHandle {
    type Target = Sftp;
    fn deref(&self) -> &Sftp { self.0.as_ref().unwrap() }
}

impl DerefMut for SftpHandle {
    fn deref_mut(&mut self) -> &mut Sftp { self.0.as_mut().unwrap() }
}

/// 기존 세션이 살아있는지 빠르게 확인 (타임아웃 일시 적용)
fn is_session_alive(session: &Session) -> bool {
    if !session.authenticated() {
        return false;
    }
    session.set_timeout(ALIVE_CHECK_TIMEOUT_MS);
    let alive = session.channel_session().is_ok();
    session.set_timeout(0); // 원복: 무제한
    alive
}

/// SshConfig를 직접 받아 SSH 연결
fn connect_inner(ssh_config: &SshConfig, force: bool) -> Result<()> {
    if !force {
        let mut client = SSH_CLIENT.lock().unwrap();
        if let Some(ref session) = *client {
            if is_session_alive(session) {
                return Ok(());
            }
        }
        // 죽은 세션 정리 — 이후 get_channel_session 등에서 블로킹 방지
        *client = None;
        *SFTP_CACHE.lock().unwrap() = None;
    } else {
        // force: 기존 세션 즉시 정리
        *SSH_CLIENT.lock().unwrap() = None;
        *SFTP_CACHE.lock().unwrap() = None;
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
        is_session_alive(session)
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

pub fn get_sftp_session() -> Result<SftpHandle> {
    // 캐시에서 꺼내기
    let cached = SFTP_CACHE.lock().unwrap().take();
    if let Some(sftp) = cached {
        // 살아있는지 간단 확인
        if sftp.stat(Path::new(".")).is_ok() {
            return Ok(SftpHandle(Some(sftp)));
        }
    }
    // 새로 생성
    let sftp = SSH_CLIENT.lock().unwrap()
        .as_ref()
        .context("SSH session not initialized")?
        .sftp().context("Failed to open SFTP session")?;
    Ok(SftpHandle(Some(sftp)))
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

// ── Content Search ──

#[derive(Debug, Clone, Serialize)]
pub struct SearchMatch {
    pub file_path: String,
    pub line_num: u32,
    pub line_text: String,
    pub is_hidden: bool,
}

/// Shell-escape a string for use inside single quotes.
fn shell_escape(s: &str) -> String {
    // Wrap in single quotes; escape any embedded single quotes.
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Search Hugo content (both public + hidden) via SSH grep.
pub fn search_content(query: &str) -> Result<Vec<SearchMatch>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let hugo = get_hugo_config()?;
    let mut channel = get_channel_session()?;

    let escaped = shell_escape(query);
    let content_dir = format!("{}/content", hugo.base_path);
    let cmd = format!(
        "grep -rn --include='*.md' -F -- {} {} 2>/dev/null || true",
        escaped, content_dir
    );

    let output = execute_ssh_command(&mut channel, &cmd)?;
    let prefix = format!("{}/content", hugo.base_path);
    let hidden_prefix = format!("/{}", hugo.hidden_path);
    let results = parse_grep_output(&output, &prefix, &hidden_prefix);
    Ok(results)
}

/// Parse grep -rn output lines into SearchMatch vec.
/// Each line: `/abs/path/content/blog/post/_index.md:12:matched text`
fn parse_grep_output(output: &str, prefix: &str, hidden_prefix: &str) -> Vec<SearchMatch> {
    let mut results = Vec::new();
    for line in output.lines() {
        // Split at first two colons: path:linenum:text
        let Some((path, rest)) = line.split_once(':') else { continue };
        let Some((num_str, text)) = rest.split_once(':') else { continue };
        let Ok(line_num) = num_str.parse::<u32>() else { continue };

        // Strip base prefix to get relative path like "/blog/post/_index.md"
        // or "/{hidden_path}/blog/post/_index.md" for hidden files
        let rel = if let Some(stripped) = path.strip_prefix(prefix) {
            stripped.to_string()
        } else {
            path.to_string()
        };

        // Detect hidden files and strip hidden_path prefix
        let (file_path, is_hidden) = if let Some(stripped) = rel.strip_prefix(hidden_prefix) {
            (stripped.to_string(), true)
        } else {
            (rel, false)
        };

        results.push(SearchMatch {
            file_path,
            line_num,
            line_text: text.trim().to_string(),
            is_hidden,
        });
    }
    results
}
