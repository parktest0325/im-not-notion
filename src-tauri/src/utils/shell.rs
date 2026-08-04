/// POSIX 셸 single-quote 이스케이프.
/// 원격 명령 문자열에 삽입되는 모든 경로/인자에 사용한다.
pub fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
