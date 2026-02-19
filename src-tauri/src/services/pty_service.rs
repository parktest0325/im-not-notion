use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Mutex;
use std::time::Duration;
use ssh2::Session;
use anyhow::{Result, Context};
use once_cell::sync::Lazy;

const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

struct PtyState {
    session: Session,
    channel: ssh2::Channel,
    running: bool,
}

static PTY_STATE: Lazy<Mutex<Option<PtyState>>> = Lazy::new(|| Mutex::new(None));

/// PTY 세션 시작 (기존 SSH_CLIENT와 별도 연결)
pub fn start_pty(host: &str, port: &str, username: &str, password: &str, cols: u32, rows: u32) -> Result<()> {
    // 기존 PTY가 있으면 먼저 정리
    stop_pty().ok();

    // 별도 SSH 세션 생성
    let mut session = Session::new().context("Failed to create PTY SSH session")?;
    let addr = format!("{}:{}", host, port);
    let sock_addr = addr.to_socket_addrs()
        .context("Failed to resolve SSH address for PTY")?
        .next()
        .context("No address found for PTY SSH host")?;
    let tcp = TcpStream::connect_timeout(&sock_addr, TCP_CONNECT_TIMEOUT)
        .context("Failed to connect to SSH server for PTY (timeout)")?;
    session.set_tcp_stream(tcp);
    session.handshake().context("PTY SSH handshake failed")?;
    session.userauth_password(username, password)
        .context("PTY SSH authentication failed")?;

    // PTY 할당 + 쉘 시작 (blocking 모드에서)
    let mut channel = session.channel_session().context("Failed to open PTY channel")?;
    channel.request_pty("xterm-256color", None, Some((cols, rows, 0, 0)))
        .context("Failed to request PTY")?;
    channel.shell().context("Failed to start shell")?;

    // 비블로킹 전환 (읽기 루프용)
    session.set_blocking(false);

    *PTY_STATE.lock().unwrap() = Some(PtyState {
        session,
        channel,
        running: true,
    });

    Ok(())
}

/// PTY에서 데이터 읽기 (비블로킹)
pub fn read_pty(buf: &mut [u8]) -> Result<usize> {
    let mut guard = PTY_STATE.lock().unwrap();
    let state = guard.as_mut().context("PTY not started")?;

    if !state.running {
        return Ok(0);
    }

    match state.channel.read(buf) {
        Ok(n) => Ok(n),
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(0),
        Err(e) => Err(e.into()),
    }
}

/// PTY에 데이터 쓰기
pub fn write_pty(data: &[u8]) -> Result<()> {
    let mut guard = PTY_STATE.lock().unwrap();
    let state = guard.as_mut().context("PTY not started")?;

    // 쓰기는 일시적으로 blocking
    state.session.set_blocking(true);
    state.channel.write_all(data).context("Failed to write to PTY")?;
    state.channel.flush().context("Failed to flush PTY")?;
    state.session.set_blocking(false);

    Ok(())
}

/// PTY 크기 변경
pub fn resize_pty(cols: u32, rows: u32) -> Result<()> {
    let mut guard = PTY_STATE.lock().unwrap();
    let state = guard.as_mut().context("PTY not started")?;

    state.session.set_blocking(true);
    state.channel.request_pty_size(cols, rows, None, None)
        .context("Failed to resize PTY")?;
    state.session.set_blocking(false);

    Ok(())
}

/// PTY 세션 종료
pub fn stop_pty() -> Result<()> {
    let mut guard = PTY_STATE.lock().unwrap();
    if let Some(mut state) = guard.take() {
        state.running = false;
        state.session.set_blocking(true);
        // channel close는 에러 무시 (이미 닫혔을 수 있음)
        let _ = state.channel.close();
        let _ = state.session.disconnect(None, "PTY closed", None);
    }
    Ok(())
}

/// PTY가 실행 중인지 확인
pub fn is_pty_running() -> bool {
    PTY_STATE.lock().unwrap()
        .as_ref()
        .map_or(false, |s| s.running)
}
