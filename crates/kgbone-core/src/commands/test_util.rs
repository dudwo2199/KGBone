//! 명령 테스트 도우미: `Document::execute`를 JSON 글자 인자로 부른다.
use crate::json::{parse, JValue};
use crate::{Document, Result};

/// 명령을 실행하고 결과(또는 오류)를 돌려준다.
pub(crate) fn exec(doc: &mut Document, id: &str, args: &str) -> Result<JValue> {
    doc.execute(id, &parse(args).expect("인자 JSON"))
}

/// 명령을 실행한다. 실패하면 테스트를 멈춘다.
pub(crate) fn run(doc: &mut Document, id: &str, args: &str) -> JValue {
    exec(doc, id, args).unwrap_or_else(|e| panic!("{id} 실패: {e}"))
}

/// 실수 비교(1e-3).
pub(crate) fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}
