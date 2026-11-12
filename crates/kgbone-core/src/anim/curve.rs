//! 키 사이 보간(선형·계단·베지어).
//!
//! 규칙(FORMAT.md 3.7):
//! - 첫 키 앞은 첫 키 값, 마지막 키 뒤는 마지막 키 값.
//! - 앞 키의 `curve`가 두 키 사이 모양을 정한다: 선형 / 계단(다음 키 전까지 앞 값) / 베지어.
//! - 베지어는 값 채널마다 `[cx1, cy1, cx2, cy2]`(절대 시간·값 좌표). 그 채널의 곡선이 없으면 선형으로 본다.
use crate::model::{Curve, Key};

/// 시점이 키 목록의 어디에 있는지.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Segment {
    /// 키가 없음.
    Empty,
    /// 이 키 값을 그대로 쓴다(첫 키 앞, 마지막 키 뒤, 키와 같은 시점).
    At(usize),
    /// `i`번 키와 `i + 1`번 키 사이.
    Between(usize),
}

/// `time`이 들어가는 구간을 찾는다(이분 탐색).
pub(crate) fn segment(keys: &[Key], time: f32) -> Segment {
    if keys.is_empty() {
        return Segment::Empty;
    }
    if time <= keys[0].time {
        return Segment::At(0);
    }
    let last = keys.len() - 1;
    if time >= keys[last].time {
        return Segment::At(last);
    }
    // keys[lo].time <= time < keys[hi].time 를 유지
    let (mut lo, mut hi) = (0usize, last);
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if keys[mid].time <= time {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Segment::Between(lo)
}

/// 시점에서 앞쪽(같거나 이전) 키 번호. 첫 키 앞이면 0.
pub(crate) fn step_index(keys: &[Key], time: f32) -> Option<usize> {
    match segment(keys, time) {
        Segment::Empty => None,
        Segment::At(i) | Segment::Between(i) => Some(i),
    }
}

fn channel(k: &Key, ch: usize) -> f32 {
    k.v.get(ch).copied().unwrap_or(0.0)
}

/// `keys`(시간순)의 `channel`번째 값을 시점 `time`에서 구한다. 첫 키 앞은 첫 키 값, 마지막 키 뒤는 마지막 값.
pub fn value_at(keys: &[Key], channel_index: usize, time: f32) -> f32 {
    match segment(keys, time) {
        Segment::Empty => 0.0,
        Segment::At(i) => channel(&keys[i], channel_index),
        Segment::Between(i) => {
            let (k0, k1) = (&keys[i], &keys[i + 1]);
            let (v0, v1) = (channel(k0, channel_index), channel(k1, channel_index));
            match &k0.curve {
                Curve::Stepped => v0,
                Curve::Linear => lerp_time(k0.time, v0, k1.time, v1, time),
                Curve::Bezier(c) => match c.get(channel_index * 4..channel_index * 4 + 4) {
                    Some(cc) => bezier_value(k0.time, v0, k1.time, v1, [cc[0], cc[1], cc[2], cc[3]], time),
                    None => lerp_time(k0.time, v0, k1.time, v1, time),
                },
            }
        }
    }
}

/// 두 키 사이 진행 비율(0~1). deform처럼 값이 아주 많은 타임라인이 쓴다.
/// 베지어는 첫 채널 `[cx1, cy1, cx2, cy2]`를 (t0, 0) – (t1, 1) 사이의 비율 곡선으로 본다(cy는 0~1).
/// 돌려주는 값: (앞 키 번호, 비율). 비율 0이면 앞 키 값 그대로.
pub(crate) fn ratio_at(keys: &[Key], time: f32) -> Option<(usize, f32)> {
    match segment(keys, time) {
        Segment::Empty => None,
        Segment::At(i) => Some((i, 0.0)),
        Segment::Between(i) => {
            let (k0, k1) = (&keys[i], &keys[i + 1]);
            let r = match &k0.curve {
                Curve::Stepped => 0.0,
                Curve::Linear => lerp_time(k0.time, 0.0, k1.time, 1.0, time),
                Curve::Bezier(c) if c.len() >= 4 => bezier_value(k0.time, 0.0, k1.time, 1.0, [c[0], c[1], c[2], c[3]], time),
                Curve::Bezier(_) => lerp_time(k0.time, 0.0, k1.time, 1.0, time),
            };
            Some((i, r))
        }
    }
}

fn lerp_time(t0: f32, v0: f32, t1: f32, v1: f32, time: f32) -> f32 {
    let span = t1 - t0;
    if span <= 0.0 {
        return v1;
    }
    let p = ((time - t0) / span).clamp(0.0, 1.0);
    v0 + (v1 - v0) * p
}

/// 3차 베지어 한 성분: 끝점 p0, p3, 제어점 p1, p2, 매개변수 s.
fn cubic(p0: f64, p1: f64, p2: f64, p3: f64, s: f64) -> f64 {
    let u = 1.0 - s;
    u * u * u * p0 + 3.0 * u * u * s * p1 + 3.0 * u * s * s * p2 + s * s * s * p3
}

/// 베지어 곡선 [cx1, cy1, cx2, cy2] (절대 좌표)로 두 키 (t0, v0) – (t1, v1) 사이 시점 `time`의 값을 구한다.
///
/// 시간 성분 x(s) = time 을 s에 대해 이분법으로 푼 뒤(제어점 시간은 [t0, t1]로 잘라 x(s)가 단조 증가하게 한다)
/// 값 성분 y(s)를 돌려준다. 계산은 f64로 해서 끝자리 흔들림을 줄인다.
pub fn bezier_value(t0: f32, v0: f32, t1: f32, v1: f32, c: [f32; 4], time: f32) -> f32 {
    if t1 <= t0 {
        return v1;
    }
    if time <= t0 {
        return v0;
    }
    if time >= t1 {
        return v1;
    }
    let (t0d, t1d) = (t0 as f64, t1 as f64);
    let cx1 = (c[0] as f64).clamp(t0d, t1d);
    let cx2 = (c[2] as f64).clamp(t0d, t1d);
    let target = time as f64;
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if cubic(t0d, cx1, cx2, t1d, mid) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let s = 0.5 * (lo + hi);
    cubic(v0 as f64, c[1] as f64, c[3] as f64, v1 as f64, s) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(time: f32, v: f32, curve: Curve) -> Key {
        Key { time, v: vec![v], curve, ..Default::default() }
    }

    #[test]
    fn f_g08_linear_stepped_bezier() {
        // 선형: (0, 0) → (2, 10), t = 0.5 → 10 × 0.5/2 = 2.5
        let lin = vec![key(0.0, 0.0, Curve::Linear), key(2.0, 10.0, Curve::Linear)];
        assert!((value_at(&lin, 0, 0.5) - 2.5).abs() < 1e-5);
        // 첫 키 앞 = 첫 값, 마지막 뒤 = 마지막 값
        assert_eq!(value_at(&lin, 0, -1.0), 0.0);
        assert_eq!(value_at(&lin, 0, 5.0), 10.0);
        // 계단: 다음 키(2초) 전까지 앞 값 0, 2초부터 10
        let st = vec![key(0.0, 0.0, Curve::Stepped), key(2.0, 10.0, Curve::Linear)];
        assert_eq!(value_at(&st, 0, 1.999), 0.0);
        assert_eq!(value_at(&st, 0, 2.0), 10.0);
        // 베지어 (0.25, 0, 0.75, 1): 두 키 (0, 0) – (1, 1). 곡선이 점 (0.5, 0.5)에 대해 대칭이므로
        // x(s) = 0.5 의 해는 s = 0.5, y(0.5) = 3·0.25·0.5·0 + 3·0.5·0.25·1 + 0.125 = 0.375 + 0.125 = 0.5
        let bz = vec![key(0.0, 0.0, Curve::Bezier(vec![0.25, 0.0, 0.75, 1.0])), key(1.0, 1.0, Curve::Linear)];
        assert!((value_at(&bz, 0, 0.5) - 0.5).abs() < 1e-5);
        // 대칭: y(0.25) + y(0.75) = 1
        let a = value_at(&bz, 0, 0.25);
        let b = value_at(&bz, 0, 0.75);
        assert!((a + b - 1.0).abs() < 1e-4, "{a} {b}");
        assert!(a < 0.25, "느리게 시작해야 함: {a}");
        // 제어점이 직선 위면 선형과 같다: (1/3, 1/3), (2/3, 2/3) → t = 0.3 에서 0.3
        let lin_bz = vec![key(0.0, 0.0, Curve::Bezier(vec![1.0 / 3.0, 1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0])), key(1.0, 1.0, Curve::Linear)];
        assert!((value_at(&lin_bz, 0, 0.3) - 0.3).abs() < 1e-4);
    }

    #[test]
    fn f_g08_bezier_per_channel() {
        // 2채널 키: 채널 0은 선형과 같은 곡선, 채널 1은 계단 같은 곡선이 아니라 (0.25,0,0.75,값) 대칭 곡선
        let k0 = Key { time: 0.0, v: vec![0.0, 0.0], curve: Curve::Bezier(vec![1.0 / 3.0, 10.0 / 3.0, 2.0 / 3.0, 20.0 / 3.0, 0.25, 0.0, 0.75, 4.0]), ..Default::default() };
        let k1 = Key { time: 1.0, v: vec![10.0, 4.0], ..Default::default() };
        let keys = vec![k0, k1];
        assert!((value_at(&keys, 0, 0.5) - 5.0).abs() < 1e-3);
        assert!((value_at(&keys, 1, 0.5) - 2.0).abs() < 1e-3);
    }

    #[test]
    fn ratio_for_deform() {
        let keys = vec![key(0.0, 0.0, Curve::Linear), key(2.0, 0.0, Curve::Linear)];
        assert_eq!(ratio_at(&keys, 0.5), Some((0, 0.25)));
        assert_eq!(ratio_at(&keys, 3.0), Some((1, 0.0)));
    }
}
