//! 구간 안의 이벤트·물리 reset 키 찾기.
//!
//! 구간은 [last, time) (끝 제외). 반복 재생(looping)이고 길이 d > 0이면:
//! - last ≤ time: 실제 시간축에서 매 바퀴 k의 (키 시간 + k·d)가 [last, time)에 드는 것.
//! - last > time(호출하는 쪽이 이미 길이로 나눈 시점을 준 경우): [last, d] ∪ [0, time).
use super::FiredEvent;
use crate::model::{Animation, Skeleton};

/// 키 시간들 중 구간에 드는 것들을 (시간 순서로) 돌려준다. 같은 키가 여러 바퀴에 들면 여러 번.
fn hits(times: &[f32], last: f32, time: f32, looping: bool, duration: f32) -> Vec<(usize, f32)> {
    let mut out = vec![];
    if looping && duration > 0.0 {
        if last <= time {
            let k0 = (last / duration).floor() as i64;
            let k1 = (time / duration).floor() as i64;
            for k in k0..=k1 {
                for (i, &t) in times.iter().enumerate() {
                    let at = t + k as f32 * duration;
                    // 길이와 같은 시간의 키는 다음 바퀴의 0초 키와 같은 순간에 함께 발생할 수 있다.
                    if at >= last && at < time {
                        out.push((i, at));
                    }
                }
            }
        } else {
            for (i, &t) in times.iter().enumerate() {
                if t >= last && t <= duration {
                    out.push((i, t));
                }
            }
            for (i, &t) in times.iter().enumerate() {
                if t < time {
                    out.push((i, t));
                }
            }
        }
    } else {
        for (i, &t) in times.iter().enumerate() {
            if t >= last && t < time {
                out.push((i, t));
            }
        }
    }
    out
}

/// 구간 [last, time)에서 발생한 이벤트. 키에 없는 값은 이벤트 정의의 기본값.
pub(crate) fn fired(skel: &Skeleton, anim: &Animation, last: f32, time: f32, looping: bool) -> Vec<FiredEvent> {
    let times: Vec<f32> = anim.events.iter().map(|e| e.time).collect();
    hits(&times, last, time, looping, anim.duration())
        .into_iter()
        .map(|(i, _)| {
            let k = &anim.events[i];
            let def = skel.event(&k.name);
            FiredEvent {
                name: k.name.clone(),
                time: k.time,
                int: k.int.or(def.map(|d| d.int)).unwrap_or(0),
                float: k.float.or(def.map(|d| d.float)).unwrap_or(0.0),
                string: k.string.clone().or(def.map(|d| d.string.clone())).unwrap_or_default(),
                volume: k.volume.or(def.map(|d| d.volume)).unwrap_or(1.0),
                balance: k.balance.or(def.map(|d| d.balance)).unwrap_or(0.0),
            }
        })
        .collect()
}

/// 구간 안에 reset 키가 있는 물리 제약 이름들(`""` = 모든 물리 제약).
pub(crate) fn physics_resets(anim: &Animation, last: f32, time: f32, looping: bool) -> Vec<String> {
    let d = anim.duration();
    let mut out = vec![];
    for (name, tls) in anim.constraints.iter() {
        if let Some(keys) = tls.get("reset") {
            let times: Vec<f32> = keys.iter().map(|k| k.time).collect();
            if !hits(&times, last, time, looping, d).is_empty() {
                out.push(name.clone());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::model::{Animation, EventDef, EventKey, Skeleton};
    use crate::pose::{PoseEngine, PoseRequest};

    fn skel() -> Skeleton {
        let mut s = Skeleton::new_with_root();
        let mut def = EventDef::new("step");
        def.int = 7;
        s.events.push(def);
        let mut a = Animation::new("walk");
        for t in [0.0, 0.5, 1.0] {
            a.events.push(EventKey { time: t, name: "step".into(), int: None, float: Some(t), string: None, volume: None, balance: None });
        }
        s.animations.push(a);
        s
    }

    fn fire(s: &Skeleton, last: f32, time: f32, looping: bool) -> Vec<f32> {
        let req = PoseRequest { animation: Some("walk".into()), time, last_time: Some(last), looping, ..Default::default() };
        PoseEngine::new().compute(s, &req).events.iter().map(|e| e.time).collect()
    }

    #[test]
    fn f_g07_events_in_half_open_range() {
        let s = skel();
        // [0, 0.5): 0초만(0.5는 끝이라 제외)
        assert_eq!(fire(&s, 0.0, 0.5, false), vec![0.0]);
        // [0.5, 1.0): 0.5만
        assert_eq!(fire(&s, 0.5, 1.0, false), vec![0.5]);
        // [0.2, 0.4): 없음
        assert!(fire(&s, 0.2, 0.4, false).is_empty());
        // 기본값: int는 정의 값 7, float는 키 값
        let req = PoseRequest { animation: Some("walk".into()), time: 0.1, last_time: Some(0.0), ..Default::default() };
        let e = &PoseEngine::new().compute(&s, &req).events[0];
        assert_eq!((e.int, e.float), (7, 0.0));
        // 반복: 길이 1. 실제 시간 [0.9, 1.2) → 다음 바퀴 1.0(= 0초 키)
        assert_eq!(fire(&s, 0.9, 1.2, true).len(), 2); // 1.0 키(첫 바퀴 끝) + 0초 키(둘째 바퀴 시작)
        // 이미 나눈 시점: last 0.9 > time 0.1 → [0.9, 1] ∪ [0, 0.1) → 1.0, 0.0
        assert_eq!(fire(&s, 0.9, 0.1, true), vec![1.0, 0.0]);
        // 이벤트 없음: lastTime 없으면 비어 있다
        let req = PoseRequest { animation: Some("walk".into()), time: 0.6, ..Default::default() };
        assert!(PoseEngine::new().compute(&s, &req).events.is_empty());
        let _ = Animation::new("x");
    }
}
