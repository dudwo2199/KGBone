//! 그리기 순서 키(drawOrder)의 오프셋 ↔ 슬롯 순서 변환.
//!
//! 키의 `offsets`는 "셋업 순서에서 몇 칸 옮겼는가"로 적는다(FORMAT.md 3.7). 오프셋이 없는 슬롯은
//! 옮겨진 슬롯이 차지하지 않은 자리를 셋업 순서대로 채운다.
use crate::model::{DrawOrderKey, DrawOrderOffset, Skeleton};
use crate::Result;

/// 셋업 슬롯 이름 순서.
pub(super) fn setup_names(skel: &Skeleton) -> Vec<String> {
    skel.slots.iter().map(|s| s.name.clone()).collect()
}

/// 키 하나를 슬롯 이름 순서로 푼다.
pub(super) fn decode(setup: &[String], offsets: &[DrawOrderOffset]) -> Vec<String> {
    let n = setup.len();
    let mut moved: Vec<(usize, i32)> =
        offsets.iter().filter_map(|o| setup.iter().position(|s| *s == o.slot).map(|i| (i, o.offset))).collect();
    moved.sort_by_key(|m| m.0);
    moved.dedup_by_key(|m| m.0);
    let mut order: Vec<Option<usize>> = vec![None; n];
    let mut unchanged = vec![];
    let mut orig = 0usize;
    for (si, off) in moved {
        while orig < si {
            unchanged.push(orig);
            orig += 1;
        }
        let target = orig as i64 + off as i64;
        if target >= 0 && (target as usize) < n && order[target as usize].is_none() {
            order[target as usize] = Some(orig);
        } else {
            unchanged.push(orig);
        }
        orig += 1;
    }
    while orig < n {
        unchanged.push(orig);
        orig += 1;
    }
    for i in (0..n).rev() {
        if order[i].is_none() {
            order[i] = unchanged.pop();
        }
    }
    order.into_iter().flatten().map(|i| setup[i].clone()).collect()
}

/// 원하는 순서 `order`를 셋업 기준 오프셋으로 적는다(자리가 바뀐 슬롯만).
/// 자리가 같은 슬롯은 빈자리를 셋업 순서대로 채우므로 정확히 제자리로 돌아간다.
pub(super) fn encode(setup: &[String], order: &[String]) -> Vec<DrawOrderOffset> {
    let mut out = vec![];
    for (si, name) in setup.iter().enumerate() {
        if let Some(pos) = order.iter().position(|o| o == name) {
            if pos != si {
                out.push(DrawOrderOffset { slot: name.clone(), offset: pos as i32 - si as i32 });
            }
        }
    }
    out
}

/// `time` 시점의 그리기 순서(그 시점 이전 마지막 키, 없으면 셋업 순서).
pub(super) fn order_at(skel: &Skeleton, keys: &[DrawOrderKey], time: f32) -> Vec<String> {
    let setup = setup_names(skel);
    match keys.iter().rev().find(|k| k.time <= time + super::keying::TIME_EPS) {
        Some(k) => decode(&setup, &k.offsets),
        None => setup,
    }
}

/// 셋업 슬롯 배열을 바꾸는 편집 `f`를 하면서, 모든 애니메이션의 그리기 순서 키가 뜻하는 순서를 유지한다.
/// 사라진 슬롯은 순서에서 빠진다.
pub(super) fn preserving_keys<T>(skel: &mut Skeleton, f: impl FnOnce(&mut Skeleton) -> Result<T>) -> Result<T> {
    let setup = setup_names(skel);
    let decoded: Vec<Vec<Vec<String>>> =
        skel.animations.iter().map(|a| a.draw_order.iter().map(|k| decode(&setup, &k.offsets)).collect()).collect();
    let out = f(skel)?;
    let new_setup = setup_names(skel);
    for (anim, orders) in skel.animations.iter_mut().zip(decoded) {
        for (key, order) in anim.draw_order.iter_mut().zip(orders) {
            let mut order: Vec<String> = order.into_iter().filter(|n| new_setup.contains(n)).collect();
            // 새로 생긴 슬롯은 셋업 자리 그대로 끼운다.
            for (i, n) in new_setup.iter().enumerate() {
                if !order.contains(n) {
                    order.insert(i.min(order.len()), n.clone());
                }
            }
            key.offsets = encode(&new_setup, &order);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn encode_decode_roundtrip() {
        let setup = names(&["a", "b", "c", "d"]);
        for order in [["b", "a", "c", "d"], ["d", "c", "b", "a"], ["a", "c", "d", "b"], ["a", "b", "c", "d"]] {
            let order = names(&order);
            let off = encode(&setup, &order);
            assert_eq!(decode(&setup, &off), order);
        }
        // a를 맨 위(끝)로: a의 오프셋 +3
        let off = encode(&setup, &names(&["b", "c", "d", "a"]));
        assert_eq!(off, vec![DrawOrderOffset { slot: "a".into(), offset: 3 }, DrawOrderOffset { slot: "b".into(), offset: -1 }, DrawOrderOffset { slot: "c".into(), offset: -1 }, DrawOrderOffset { slot: "d".into(), offset: -1 }]);
    }
}
