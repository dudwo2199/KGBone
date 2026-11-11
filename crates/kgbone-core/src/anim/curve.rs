//! 키 사이 보간(선형·계단·베지어).
use crate::model::Key;

/// `keys`(시간순)의 `channel`번째 값을 시점 `time`에서 구한다. 첫 키 앞은 첫 키 값, 마지막 키 뒤는 마지막 값.
pub fn value_at(keys: &[Key], channel: usize, time: f32) -> f32 {
    let _ = (keys, channel, time);
    0.0
}

/// 베지어 곡선 [cx1, cy1, cx2, cy2] (절대 좌표)로 두 키 (t0, v0) – (t1, v1) 사이 시점 `time`의 값을 구한다.
pub fn bezier_value(t0: f32, v0: f32, t1: f32, v1: f32, c: [f32; 4], time: f32) -> f32 {
    let _ = (t0, v0, t1, v1, c, time);
    0.0
}
