use ssh2::{Session, Channel, Sftp};
use std::{net::{TcpStream, ToSocketAddrs}, sync::Mutex, sync::atomic::{AtomicU64, Ordering}, io::Read, time::Duration, path::Path, ops::{Deref, DerefMut}};
use anyhow::{Result, Context};
use serde::Serialize;
use crate::types::config::SshConfig;
use crate::services::config_service::get_hugo_config;
use once_cell::sync::Lazy;

const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const ALIVE_CHECK_TIMEOUT_MS: u32 = 2000;

static SSH_CLIENT: Lazy<Mutex<Option<Session>>> = Lazy::new(|| Mutex::new(None));
static SFTP_CACHE: Lazy<Mutex<Option<Sftp>>> = Lazy::new(|| Mutex::new(None));
// 재연결 시 증가 — 이전 세션에서 대여된 SftpHandle이 죽은 Sftp를
// 캐시에 되돌려 넣는 것을 방지한다
static SSH_GENERATION: AtomicU64 = AtomicU64::new(0);

/// RAII wrapper: drop 시 SFTP 세션을 캐시에 반환 (같은 세션 세대일 때만)
pub struct SftpHandle(Option<Sftp>, u64);

impl Drop for SftpHandle {
    fn drop(&mut self) {
        if let Some(sftp) = self.0.take() {
            if self.1 == SSH_GENERATION.load(Ordering::SeqCst) {
                *SFTP_CACHE.lock().unwrap_or_else(|p| p.into_inner()) = Some(sftp);
            }
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
        let mut client = SSH_CLIENT.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(ref session) = *client {
            if is_session_alive(session) {
                return Ok(());
            }
        }
        // 죽은 세션 정리 — 이후 get_channel_session 등에서 블로킹 방지
        *client = None;
        *SFTP_CACHE.lock().unwrap_or_else(|p| p.into_inner()) = None;
        SSH_GENERATION.fetch_add(1, Ordering::SeqCst);
    } else {
        // force: 기존 세션 즉시 정리
        *SSH_CLIENT.lock().unwrap_or_else(|p| p.into_inner()) = None;
        *SFTP_CACHE.lock().unwrap_or_else(|p| p.into_inner()) = None;
        SSH_GENERATION.fetch_add(1, Ordering::SeqCst);
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
    // NAT 타임아웃/무단절 링크 감지용 keepalive
    session.set_keepalive(true, 30);

    if !ssh_config.password.is_empty() {
        session.userauth_password(&ssh_config.username, &ssh_config.password)
            .context("Failed to authenticate with password")?;
    }

    let mut ssh_client = SSH_CLIENT.lock().unwrap_or_else(|p| p.into_inner());
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
    let client = SSH_CLIENT.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(ref session) = *client {
        is_session_alive(session)
    } else {
        false
    }
}

pub fn get_channel_session() -> Result<Channel> {
    let channel = SSH_CLIENT.lock().unwrap_or_else(|p| p.into_inner())
        .as_ref()
        .context("SSH session not initialized")?
        .channel_session().context("Failed to open SSH channel session")?;
    Ok(channel)
}

pub fn get_sftp_session() -> Result<SftpHandle> {
    let generation = SSH_GENERATION.load(Ordering::SeqCst);
    // 캐시에서 꺼내기
    let cached = SFTP_CACHE.lock().unwrap_or_else(|p| p.into_inner()).take();
    if let Some(sftp) = cached {
        // 살아있는지 간단 확인
        if sftp.stat(Path::new(".")).is_ok() {
            return Ok(SftpHandle(Some(sftp), generation));
        }
    }
    // 새로 생성
    let sftp = SSH_CLIENT.lock().unwrap_or_else(|p| p.into_inner())
        .as_ref()
        .context("SSH session not initialized")?
        .sftp().context("Failed to open SFTP session")?;
    Ok(SftpHandle(Some(sftp), generation))
}

/// SSH 서버의 홈 디렉토리 경로를 가져옴
pub fn get_server_home_path() -> Result<String> {
    let mut channel = get_channel_session()?;
    let output = execute_ssh_command(&mut channel, "echo $HOME")?;
    Ok(output.trim().to_string())
}

/// 원격 명령 실행 + exit code 확인. 비0 종료를 stderr와 함께 에러로 전파한다.
/// (grep/pkill처럼 비0 종료가 정상인 명령에는 execute_ssh_command를 사용)
pub fn execute_ssh_command_checked(channel: &mut Channel, command: &str) -> Result<String> {
    channel.exec(command).context("Failed to execute SSH command")?;

    let mut stdout = String::new();
    channel.read_to_string(&mut stdout).context("Failed to read from SSH stdout")?;

    let mut stderr = String::new();
    channel.stderr().read_to_string(&mut stderr).context("Failed to read from SSH stderr")?;

    channel.wait_close().context("Failed to close SSH channel")?;
    let exit_status = channel.exit_status().context("Failed to get SSH exit status")?;
    if exit_status != 0 {
        return Err(anyhow::anyhow!(
            "Remote command failed (exit {}): {}",
            exit_status,
            stderr.trim()
        ));
    }
    Ok(stdout)
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

use crate::utils::shell::quote as shell_escape;

/// Search Hugo content (both public + hidden) via SSH grep.
///
/// tags가 있으면 front matter의 tags 줄 기준으로 파일을 한정한다.
/// match_all=true면 모든 태그 보유(AND), false면 하나라도 보유(OR).
/// query가 함께 있으면 그 파일들 안에서만 본문 전문 검색을 수행한다.
pub fn search_content(query: &str, tags: &[String], match_all: bool) -> Result<Vec<SearchMatch>> {
    let query = query.trim();
    let tags: Vec<&str> = tags.iter().map(|t| t.trim()).filter(|t| !t.is_empty()).collect();
    if query.is_empty() && tags.is_empty() {
        return Ok(Vec::new());
    }

    let hugo = get_hugo_config()?;
    let mut channel = get_channel_session()?;

    let content_dir = shell_escape(&format!("{}/content", hugo.base_path));
    let cmd = if tags.is_empty() {
        format!(
            "grep -rn --include='*.md' -F -- {} {} 2>/dev/null || true",
            shell_escape(query),
            content_dir
        )
    } else {
        // front matter 블록의 tags 줄만 추출 후, 태그별 grep 체인으로 AND 필터
        let mut tag_filter = format!(
            "find {} -name '*.md' -exec awk 'FNR==1{{fm=0}} FNR==1&&/^(---|\\+\\+\\+)/{{fm=1;next}} fm&&/^(---|\\+\\+\\+)/{{fm=0;nextfile}} fm&&/^tags[[:space:]]*[:=]/{{print FILENAME\":\"FNR\":\"$0}}' {{}} + 2>/dev/null",
            content_dir
        );
        // 매칭은 "경로:줄번호:tags키"를 제거한 태그 내용에만 수행한다 —
        // 라인 전체에 grep을 걸면 경로에 태그 문자열이 포함된 파일이 오탐된다.
        let joined = tags.join("\n");
        let mode = if match_all { "and" } else { "or" };
        tag_filter.push_str(&format!(
            " | awk -v ts={} -v mode={} 'BEGIN{{n=split(ts,T,\"\\n\")}} {{ line=$0; sub(/^[^:]*:[0-9]*:/,\"\",line); sub(/^[[:space:]]*tags[[:space:]]*[:=]/,\"\",line); l=tolower(line); if(mode==\"and\"){{ok=1; for(i=1;i<=n;i++) if(!index(l,tolower(T[i]))){{ok=0;break}}}} else {{ok=0; for(i=1;i<=n;i++) if(index(l,tolower(T[i]))){{ok=1;break}}}} if(ok) print }}'",
            shell_escape(&joined),
            mode
        ));
        if query.is_empty() {
            // 태그만: tags 줄 자체를 결과로 반환
            format!("{} || true", tag_filter)
        } else {
            // 태그로 파일을 좁힌 뒤 그 안에서 본문 검색
            format!(
                "{} | cut -d: -f1 | sort -u | xargs -r -d '\\n' grep -Hn -F -- {} 2>/dev/null || true",
                tag_filter,
                shell_escape(query)
            )
        }
    };

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
