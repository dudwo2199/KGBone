//! 숫자 입력칸 수식.
//!
//! - `v`, `value`: 현재 값
//! - `+=5`, `-=5`, `*=2`, `/=2`: 현재 값에 연산
//! - 사칙연산, 괄호, 단항 `-`, 소수
use crate::{CoreError, Result};

/// 입력 글자 `input`을 현재 값 `current` 기준으로 계산한다.
pub fn eval(input: &str, current: f64) -> Result<f64> {
    let _ = (input, current);
    Err(CoreError::NotImplemented("수식 계산"))
}
// t
