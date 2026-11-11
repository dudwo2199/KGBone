//! PNG 시퀀스 시점 계산.

#[derive(Clone, Debug, PartialEq)]
pub struct SequenceOptions {
    pub fps: f32,
}

/// 길이 `duration`초 애니메이션을 fps로 나눈 시점 목록(0과 끝 포함: 프레임 수 = floor(duration × fps) + 1).
pub fn sequence_times(duration: f32, opts: &SequenceOptions) -> Vec<f32> {
    let _ = (duration, opts);
    vec![]
}
