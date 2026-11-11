//! Tauri 명령(interface/tauri-commands.md). 오류는 화면에 그대로 보여 줄 한국어 문자열로 돌려준다.
pub mod assoc;
pub mod fs;
pub mod images;
pub mod launch;
pub mod watch;

/// 입출력 오류를 한국어 메시지로 바꾼다.
pub(crate) fn io_error(action: &str, path: &str, err: std::io::Error) -> String {
    format!("{action} 못했습니다: {path} ({err})")
}
