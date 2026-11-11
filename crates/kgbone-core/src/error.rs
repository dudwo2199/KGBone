//! 오류와 경고. 메시지는 화면에 그대로 보여 줄 한국어로 쓴다.
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum CoreError {
    /// 파일 내용이 형식에 맞지 않음.
    Parse(String),
    /// 명령 인자가 잘못됨.
    InvalidArgs(String),
    /// 찾는 항목이 없음.
    NotFound(String),
    /// 규칙상 허용되지 않는 편집.
    Rejected(String),
    /// 모르는 명령 id.
    UnknownCommand(String),
    /// 아직 구현되지 않음.
    NotImplemented(&'static str),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Parse(m) => write!(f, "파일을 읽을 수 없습니다: {m}"),
            CoreError::InvalidArgs(m) => write!(f, "잘못된 값: {m}"),
            CoreError::NotFound(m) => write!(f, "찾을 수 없습니다: {m}"),
            CoreError::Rejected(m) => write!(f, "할 수 없습니다: {m}"),
            CoreError::UnknownCommand(m) => write!(f, "모르는 명령: {m}"),
            CoreError::NotImplemented(m) => write!(f, "아직 지원하지 않습니다: {m}"),
        }
    }
}

impl std::error::Error for CoreError {}

pub type Result<T> = std::result::Result<T, CoreError>;

/// 작업은 되었지만 사용자에게 알려야 하는 것(예: 4.3 표기 미확인 항목).
#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    /// 고정 코드(예: `spine43.slider`).
    pub code: String,
    /// 한국어 설명.
    pub message: String,
    /// 관련 항목 경로(예: `constraints/흔들기`).
    pub path: String,
}

crate::json_object!(Warning { code: "code" = String::new(), message: "message" = String::new(), path: "path" = String::new() });

impl Warning {
    pub fn new(code: &str, message: impl Into<String>, path: impl Into<String>) -> Self {
        Warning { code: code.into(), message: message.into(), path: path.into() }
    }
}
