use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use ssh2::Session;
use anyhow::{Result, Context};
use once_cell::sync::Lazy;

const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const IO_LOOP_SLEEP: Duration = Duration::from_millis(1);

/// I/O 스레드로 보내는 메시지
enum PtyMsg {
    Write(Vec<u8>),
    Resize { cols: u32, rows: u32 },
    Stop,
}

/// 외부에서 I/O 스레드를 제어하는 핸들
struct PtyHandle {
    tx: mpsc::Sender<PtyMsg>,
    io_thread: Option<thread::JoinHandle<()>>,
}

static PTY_HANDLE: Lazy<Mutex<Option<PtyHandle>>> = Lazy::new(|| Mutex::new(None));

/// PTY 세션 시작
/// on_output: I/O 스레드에서 읽은 데이터를 전달하는 콜백. false 반환 시 루프 종료.
pub fn start_pty(
    host: &str, port: &str, username: &str, password: &str,
    cols: u32, rows: u32,
    on_output: Box<dyn Fn(String) -> bool + Send>,
) -> Result<()> {
    // 기존 PTY가 있으면 먼저 정리
    stop_pty().ok();

    // SSH 세션 생성 (blocking 모드에서 연결)
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

    // PTY 할당 + 쉘 시작
    let mut channel = session.channel_session().context("Failed to open PTY channel")?;
    channel.request_pty("xterm-256color", None, Some((cols, rows, 0, 0)))
        .context("Failed to request PTY")?;
    channel.shell().context("Failed to start shell")?;

    // 비블로킹 전환
    session.set_blocking(false);

    // mpsc 채널 생성
    let (tx, rx) = mpsc::channel::<PtyMsg>();

    // 단일 I/O 스레드 — SSH 채널을 독점 소유
    let io_thread = thread::spawn(move || {
        io_loop(session, channel, rx, on_output);
    });

    *PTY_HANDLE.lock().unwrap() = Some(PtyHandle {
        tx,
        io_thread: Some(io_thread),
    });

    Ok(())
}

/// 단일 I/O 이벤트 루프 — SSH 채널을 이 스레드만 접근
fn io_loop(
    session: Session,
    mut channel: ssh2::Channel,
    rx: mpsc::Receiver<PtyMsg>,
    on_output: Box<dyn Fn(String) -> bool + Send>,
) {
    let mut read_buf = [0u8; 16384];
    let mut utf8_leftover = Vec::with_capacity(4);

    loop {
        // 1) 메시지 처리 (non-blocking)
        match rx.try_recv() {
            Ok(PtyMsg::Write(data)) => {
                // non-blocking write with retry
                let mut written = 0;
                let mut retries = 0;
                while written < data.len() {
                    match channel.write(&data[written..]) {
                        Ok(n) => {
                            written += n;
                            retries = 0;
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            retries += 1;
                            if retries > 100 {
                                break; // give up after 100ms
                            }
                            thread::sleep(IO_LOOP_SLEEP);
                        }
                        Err(_) => return, // fatal error, exit loop
                    }
                }
                // flush (best effort)
                let _ = channel.flush();
            }
            Ok(PtyMsg::Resize { cols, rows }) => {
                // resize — retry once on EAGAIN
                if let Err(e) = channel.request_pty_size(cols, rows, None, None) {
                    if e.code() == ssh2::ErrorCode::Session(-37) {
                        thread::sleep(Duration::from_millis(5));
                        let _ = channel.request_pty_size(cols, rows, None, None);
                    }
                }
            }
            Ok(PtyMsg::Stop) => break,
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {} // no message, continue
        }

        // 2) 읽기 (non-blocking)
        if channel.eof() {
            break;
        }

        match channel.read(&mut read_buf) {
            Ok(n) if n > 0 => {
                // UTF-8 경계 처리
                let mut combined;
                let data: &[u8] = if utf8_leftover.is_empty() {
                    &read_buf[..n]
                } else {
                    combined = std::mem::take(&mut utf8_leftover);
                    combined.extend_from_slice(&read_buf[..n]);
                    &combined
                };

                let (valid_end, trail_start) = find_utf8_boundary(data);

                if valid_end > 0 {
                    let text = String::from_utf8_lossy(&data[..valid_end]).to_string();
                    if !on_output(text) {
                        break; // 프론트엔드 채널 닫힘
                    }
                }

                // 남은 불완전 바이트 보관
                if trail_start < data.len() {
                    utf8_leftover.extend_from_slice(&data[trail_start..]);
                }
            }
            Ok(_) => {
                // WouldBlock — 데이터 없음, 짧게 대기
                thread::sleep(IO_LOOP_SLEEP);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(IO_LOOP_SLEEP);
            }
            Err(_) => break, // fatal error
        }
    }

    // 정리
    session.set_blocking(true);
    let _ = channel.close();
    let _ = session.disconnect(None, "PTY closed", None);
}

/// UTF-8 멀티바이트 경계를 찾아서 (유효한 끝 위치, trailing 시작 위치) 반환
fn find_utf8_boundary(data: &[u8]) -> (usize, usize) {
    if data.is_empty() {
        return (0, 0);
    }

    let len = data.len();

    for i in 1..=3.min(len) {
        let pos = len - i;
        let byte = data[pos];

        if byte < 0x80 {
            return (len, len);
        }

        let expected_len = if byte & 0xE0 == 0xC0 { 2 }
            else if byte & 0xF0 == 0xE0 { 3 }
            else if byte & 0xF8 == 0xF0 { 4 }
            else { continue };

        let available = len - pos;
        if available < expected_len {
            return (pos, pos);
        } else {
            return (len, len);
        }
    }

    (len, len)
}

/// PTY에 데이터 쓰기 (mpsc로 I/O 스레드에 전달 — 절대 블록 안 됨)
pub fn write_pty(data: &[u8]) -> Result<()> {
    let guard = PTY_HANDLE.lock().unwrap();
    let handle = guard.as_ref().context("PTY not started")?;
    handle.tx.send(PtyMsg::Write(data.to_vec()))
        .map_err(|_| anyhow::anyhow!("PTY I/O thread closed"))?;
    Ok(())
}

/// PTY 크기 변경 (mpsc로 I/O 스레드에 전달)
pub fn resize_pty(cols: u32, rows: u32) -> Result<()> {
    let guard = PTY_HANDLE.lock().unwrap();
    let handle = guard.as_ref().context("PTY not started")?;
    handle.tx.send(PtyMsg::Resize { cols, rows })
        .map_err(|_| anyhow::anyhow!("PTY I/O thread closed"))?;
    Ok(())
}

/// PTY 세션 종료
pub fn stop_pty() -> Result<()> {
    let mut guard = PTY_HANDLE.lock().unwrap();
    if let Some(mut handle) = guard.take() {
        // Stop 메시지 전송 (이미 닫혔을 수 있으므로 에러 무시)
        let _ = handle.tx.send(PtyMsg::Stop);
        // I/O 스레드 종료 대기
        if let Some(thread) = handle.io_thread.take() {
            drop(guard); // 락 해제 후 join
            let _ = thread.join();
            return Ok(());
        }
    }
    Ok(())
}

