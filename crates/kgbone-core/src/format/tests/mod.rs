//! 파일 형식 테스트(샘플은 `include_bytes!`로 넣는다: WASI에서는 파일 읽기가 불편하다).
mod atlas;
mod attachments;
mod kgbone;
mod spine;

macro_rules! sample {
    ($p:literal) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/", $p))
    };
}
pub(super) use sample;
