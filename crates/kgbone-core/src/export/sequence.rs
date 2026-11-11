//! PNG 시퀀스 시점 계산.

#[derive(Clone, Debug, PartialEq)]
pub struct SequenceOptions {
    pub fps: f32,
}

/// 길이 `duration`초 애니메이션을 fps로 나눈 시점 목록(0과 끝 포함: 프레임 수 = floor(duration × fps) + 1).
/// 길이가 0 이하이거나 fps가 양수가 아니면 시점 0 하나.
pub fn sequence_times(duration: f32, opts: &SequenceOptions) -> Vec<f32> {
    let fps = opts.fps as f64;
    let d = duration as f64;
    if !(fps > 0.0) || !(d > 0.0) || !d.is_finite() {
        return vec![0.0];
    }
    // 1e-6: 1.0 × 30처럼 정확히 떨어져야 할 곱이 29.999999…로 내려가 프레임 하나를 잃지 않게.
    let count = (d * fps + 1e-6).floor() as usize + 1;
    (0..count).map(|i| (i as f64 / fps) as f32).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k02_sequence_frame_count() {
        // 1초 × 30fps = 30 → 31장, 끝 시점 30/30 = 1.
        let t = sequence_times(1.0, &SequenceOptions { fps: 30.0 });
        assert_eq!(t.len(), 31);
        assert_eq!(t[0], 0.0);
        assert!((t[30] - 1.0).abs() < 1e-6);
        // 0.5초 × 24fps = 12 → 13장. 0.1초 × 30 = 3(부동소수 오차로 2.9999…) → 4장.
        assert_eq!(sequence_times(0.5, &SequenceOptions { fps: 24.0 }).len(), 13);
        assert_eq!(sequence_times(0.1, &SequenceOptions { fps: 30.0 }).len(), 4);
        // 1.05초 × 10 = 10.5 → floor 10 → 11장(끝 1.0초, 1.05는 넘지 않음).
        let t = sequence_times(1.05, &SequenceOptions { fps: 10.0 });
        assert_eq!(t.len(), 11);
        assert!((t[1] - 0.1).abs() < 1e-6);
        // 길이 0이나 fps 0은 한 장.
        assert_eq!(sequence_times(0.0, &SequenceOptions { fps: 30.0 }), vec![0.0]);
        assert_eq!(sequence_times(1.0, &SequenceOptions { fps: 0.0 }), vec![0.0]);
    }
}
