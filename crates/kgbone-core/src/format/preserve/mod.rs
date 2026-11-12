//! 원본 보존 쓰기(형식 공용).
//!
//! 1. [`merge`]: 원본 트리 + 모델에서 새로 만든 값 → 바뀐 부분만 바뀐 트리.
//!    [`crate::json::merge`]와 같은 규칙에, 형식이 알려 주는 구조 정보([`Schema`])를 더한다:
//!    - 이름 → 값 사전(스킨 첨부물, 애니메이션 타임라인 등)에서 모델에 없는 키는 지운다(모르는 키가 아니므로).
//!    - 이름 배열에서 같은 이름이 여러 번 나오면 차례로 짝짓고, 이름이 바뀐 원소는 같은 위치의 원소와 짝짓는다.
//!    - 사전 키 이름이 바뀌면 같은 자리를 유지한다.
//! 2. [`splice`]: 원본 텍스트에서 바뀌지 않은 부분의 글자(공백·줄바꿈·숫자 표기)를 그대로 쓰고,
//!    바뀐 값만 새로 써 넣는다.
mod merge;
mod splice;

pub(crate) use merge::merge;
pub(crate) use splice::splice;

/// 형식별 구조 정보. 경로는 객체 키 이름과 배열 원소 표시 `"#"`로 이루어진다.
pub(crate) trait Schema {
    /// 이 경로의 객체가 "이름 → 값" 사전인가(모든 키를 모델이 안다).
    fn is_dict(&self, path: &[String]) -> bool;
    /// 고정 구조 객체에서, 모델이 아는(그래서 모델 값에 없으면 지워야 하는) 키인가.
    fn owns(&self, path: &[String], key: &str) -> bool;
}

/// 경로가 패턴과 맞는가. 패턴의 `"*"`는 아무 키 하나, `"#"`는 배열 원소.
pub(crate) fn path_is(path: &[String], pattern: &[&str]) -> bool {
    path.len() == pattern.len() && path.iter().zip(pattern).all(|(p, q)| *q == "*" || p == q)
}
