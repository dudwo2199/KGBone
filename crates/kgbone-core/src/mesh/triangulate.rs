//! 단순 다각형 삼각분할(귀 자르기).
//!
//! 부호 있는 넓이로 방향(시계/반시계)을 정해 반시계로 본다. 귀 = 볼록 꼭짓점이고 그 삼각형 안에 다른 점이 없는 것.
//! 같은 자리 점·일직선 점(넓이 0 꼭짓점)은 먼저 잘라 낸다(삼각형을 만들지 않음). 꼬인 다각형처럼 귀가 없으면
//! 가장 볼록한 꼭짓점을 억지로 잘라 끝까지 진행한다.

fn cross(o: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

fn in_triangle(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    // 반시계 삼각형 기준, 경계 포함
    cross(a, b, p) >= 0.0 && cross(b, c, p) >= 0.0 && cross(c, a, p) >= 0.0
}

/// 다각형 [x0, y0, x1, y1, …](시계·반시계 무관) → 삼각형 정점 번호 3개씩(반시계).
pub fn triangulate(polygon: &[f32]) -> Vec<u32> {
    let pts: Vec<(f32, f32)> = polygon.chunks_exact(2).map(|p| (p[0], p[1])).collect();
    let n = pts.len();
    if n < 3 {
        return vec![];
    }
    let area: f32 = (0..n).map(|i| cross((0.0, 0.0), pts[i], pts[(i + 1) % n])).sum();
    let mut idx: Vec<usize> = (0..n).collect();
    if area < 0.0 {
        idx.reverse();
    }
    let scale = pts.iter().fold(0.0f32, |m, p| m.max(p.0.abs()).max(p.1.abs())).max(1.0);
    let eps = 1e-7 * scale * scale;
    let mut out = Vec::with_capacity((n - 2) * 3);
    while idx.len() > 3 {
        let m = idx.len();
        let mut cut: Option<usize> = None;
        // 1) 귀 찾기(일직선 점도 이웃 귀의 꼭짓점으로 쓰이도록 귀를 먼저 자른다)
        for i in 0..m {
            let (ia, ib, ic) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let (a, b, c) = (pts[ia], pts[ib], pts[ic]);
            if cross(a, b, c) <= eps {
                continue;
            }
            let blocked = idx.iter().any(|&j| j != ia && j != ib && j != ic && pts[j] != a && pts[j] != b && pts[j] != c && in_triangle(pts[j], a, b, c));
            if !blocked {
                cut = Some(i);
                break;
            }
        }
        // 2) 귀가 없으면 넓이 0 꼭짓점(겹친 점·일직선 점)을 삼각형 없이 제거
        if cut.is_none() {
            if let Some(i) = (0..m).find(|&i| cross(pts[idx[(i + m - 1) % m]], pts[idx[i]], pts[idx[(i + 1) % m]]).abs() <= eps) {
                idx.remove(i);
                continue;
            }
        }
        // 3) 그래도 없으면(꼬인 다각형) 가장 볼록한 꼭짓점
        let i = cut.unwrap_or_else(|| {
            (0..m)
                .max_by(|&x, &y| {
                    let f = |i: usize| cross(pts[idx[(i + m - 1) % m]], pts[idx[i]], pts[idx[(i + 1) % m]]);
                    f(x).partial_cmp(&f(y)).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(0)
        });
        let (ia, ib, ic) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
        if cross(pts[ia], pts[ib], pts[ic]) > eps {
            out.extend_from_slice(&[ia as u32, ib as u32, ic as u32]);
        }
        idx.remove(i);
    }
    if idx.len() == 3 && cross(pts[idx[0]], pts[idx[1]], pts[idx[2]]) > eps {
        out.extend(idx.iter().map(|&i| i as u32));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area_of(poly: &[f32], tris: &[u32]) -> f32 {
        let p = |i: u32| (poly[2 * i as usize], poly[2 * i as usize + 1]);
        tris.chunks(3).map(|t| cross(p(t[0]), p(t[1]), p(t[2])) * 0.5).sum()
    }

    #[test]
    fn concave_both_orientations_and_degenerate() {
        // L자 넓이 = 20·5 + 5·15 = 175
        let l = [0.0, 0.0, 20.0, 0.0, 20.0, 5.0, 5.0, 5.0, 5.0, 20.0, 0.0, 20.0];
        let t = triangulate(&l);
        assert_eq!(t.len(), 12);
        assert!((area_of(&l, &t) - 175.0).abs() < 1e-3);
        let mut rev: Vec<f32> = l.chunks(2).rev().flatten().copied().collect();
        let t2 = triangulate(&rev);
        assert!((area_of(&rev, &t2) - 175.0).abs() < 1e-3);
        // 일직선 점·겹친 점이 있어도 넓이 0 삼각형 없음
        rev = vec![0.0, 0.0, 5.0, 0.0, 10.0, 0.0, 10.0, 10.0, 10.0, 10.0, 0.0, 10.0];
        let t3 = triangulate(&rev);
        assert!((area_of(&rev, &t3) - 100.0).abs() < 1e-3);
        for tri in t3.chunks(3) {
            assert!(area_of(&rev, tri) > 0.0);
        }
    }
}
