//! 클리핑: 삼각형을 클립 다각형으로 자르기(반전·볼록 옵션).
//!
//! - 클립 다각형(월드)을 볼록 조각들로 나눈다: convex면 볼록 껍질 하나, 아니면 귀 자르기 삼각형들(서로 겹치지 않음).
//! - 보통: 그려질 삼각형 × 각 조각을 Sutherland–Hodgman 방식(반평면으로 차례로 자르기)으로 교집합.
//! - 반전(inverse): 삼각형에서 각 조각을 차례로 빼낸다. 볼록 조각 C를 빼는 방법:
//!   C의 변 e₁…eₖ에 대해 "앞 변들의 안쪽이면서 eⱼ의 바깥쪽"인 부분들로 나누면 서로 겹치지 않는 볼록 조각들이 된다.
//! - 결과 다각형은 부채꼴로 삼각형화하고, uv는 원래 삼각형의 무게중심 좌표로 다시 구한다. 넓이 0인 삼각형은 버린다.
type P = (f32, f32);

const AREA_EPS: f32 = 1e-6;

pub(crate) struct Clipper {
    /// 반시계 볼록 조각들.
    pieces: Vec<Vec<P>>,
    inverse: bool,
}

fn cross(o: P, a: P, b: P) -> f32 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

fn signed_area(poly: &[P]) -> f32 {
    let n = poly.len();
    (0..n).map(|i| {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        a.0 * b.1 - b.0 * a.1
    }).sum::<f32>() * 0.5
}

/// 볼록 껍질(반시계, 단조 사슬).
pub(crate) fn convex_hull(points: &[P]) -> Vec<P> {
    let mut pts = points.to_vec();
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)));
    pts.dedup();
    if pts.len() < 3 {
        return pts;
    }
    let mut lower: Vec<P> = vec![];
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<P> = vec![];
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// 반평면 자르기: 변 a→b의 왼쪽(keep_left) 또는 오른쪽만 남긴다.
fn clip_half(poly: &[P], a: P, b: P, keep_left: bool) -> Vec<P> {
    let side = |p: P| {
        let c = cross(a, b, p);
        if keep_left {
            c
        } else {
            -c
        }
    };
    let n = poly.len();
    let mut out = Vec::with_capacity(n + 2);
    for i in 0..n {
        let (cur, nxt) = (poly[i], poly[(i + 1) % n]);
        let (sc, sn) = (side(cur), side(nxt));
        if sc >= 0.0 {
            out.push(cur);
        }
        if (sc >= 0.0) != (sn >= 0.0) {
            let t = sc / (sc - sn);
            out.push((cur.0 + (nxt.0 - cur.0) * t, cur.1 + (nxt.1 - cur.1) * t));
        }
    }
    out
}

fn intersect_convex(poly: &[P], c: &[P]) -> Vec<P> {
    let mut cur = poly.to_vec();
    for i in 0..c.len() {
        if cur.len() < 3 {
            return vec![];
        }
        cur = clip_half(&cur, c[i], c[(i + 1) % c.len()], true);
    }
    cur
}

/// 볼록 다각형 `poly`에서 볼록 조각 `c`를 뺀 나머지(서로 겹치지 않는 볼록 조각들).
fn subtract_convex(poly: &[P], c: &[P]) -> Vec<Vec<P>> {
    let mut out = vec![];
    let mut rem = poly.to_vec();
    for i in 0..c.len() {
        if rem.len() < 3 {
            break;
        }
        let (a, b) = (c[i], c[(i + 1) % c.len()]);
        let outside = clip_half(&rem, a, b, false);
        if outside.len() >= 3 && signed_area(&outside).abs() > AREA_EPS {
            out.push(outside);
        }
        rem = clip_half(&rem, a, b, true);
    }
    out
}

impl Clipper {
    /// 클립 다각형(월드 [x, y, …])으로 만든다. 점이 3개 미만이면 None.
    pub(crate) fn new(polygon: &[f32], convex: bool, inverse: bool) -> Option<Clipper> {
        let pts: Vec<P> = polygon.chunks_exact(2).map(|p| (p[0], p[1])).collect();
        if pts.len() < 3 {
            return None;
        }
        let pieces = if convex {
            vec![convex_hull(&pts)]
        } else {
            let tri = crate::mesh::triangulate(polygon);
            tri.chunks_exact(3)
                .map(|t| {
                    let mut p = vec![pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]];
                    if signed_area(&p) < 0.0 {
                        p.reverse();
                    }
                    p
                })
                .filter(|p| signed_area(p).abs() > AREA_EPS)
                .collect()
        };
        Some(Clipper { pieces, inverse })
    }

    /// 삼각형 묶음을 자른다. 돌려주는 값: (위치, uv, 번호).
    pub(crate) fn clip(&self, positions: &[f32], uvs: &[f32], indices: &[u32]) -> (Vec<f32>, Vec<f32>, Vec<u32>) {
        let (mut op, mut ou, mut oi) = (vec![], vec![], vec![]);
        let pt = |i: u32| (positions[2 * i as usize], positions[2 * i as usize + 1]);
        let uv = |i: u32| (uvs.get(2 * i as usize).copied().unwrap_or(0.0), uvs.get(2 * i as usize + 1).copied().unwrap_or(0.0));
        for t in indices.chunks_exact(3) {
            if t.iter().any(|&i| 2 * i as usize + 1 >= positions.len()) {
                continue;
            }
            let (a, b, c) = (pt(t[0]), pt(t[1]), pt(t[2]));
            let area = cross(a, b, c);
            if area.abs() <= AREA_EPS {
                continue;
            }
            let mut tri = vec![a, b, c];
            if area < 0.0 {
                tri.reverse();
            }
            let polys: Vec<Vec<P>> = if self.inverse {
                let mut rem = vec![tri];
                for piece in &self.pieces {
                    rem = rem.iter().flat_map(|r| subtract_convex(r, piece)).collect();
                }
                rem
            } else {
                self.pieces.iter().map(|piece| intersect_convex(&tri, piece)).filter(|p| p.len() >= 3).collect()
            };
            let (ua, ub, uc) = (uv(t[0]), uv(t[1]), uv(t[2]));
            for poly in polys {
                let base = (op.len() / 2) as u32;
                for &p in &poly {
                    // 무게중심 좌표(원래 순서 a, b, c 기준)
                    let wa = cross(p, b, c) / area;
                    let wb = cross(a, p, c) / area;
                    let wc = 1.0 - wa - wb;
                    op.push(p.0);
                    op.push(p.1);
                    ou.push(ua.0 * wa + ub.0 * wb + uc.0 * wc);
                    ou.push(ua.1 * wa + ub.1 * wb + uc.1 * wc);
                }
                for k in 1..poly.len() as u32 - 1 {
                    if cross(poly[0], poly[k as usize], poly[k as usize + 1]).abs() > AREA_EPS {
                        oi.extend_from_slice(&[base, base + k, base + k + 1]);
                    }
                }
            }
        }
        (op, ou, oi)
    }
}

/// 점이 볼록하지 않을 수 있는 다각형 안에 있는가(짝홀 규칙). 테스트·도구용.
#[cfg(test)]
pub(crate) fn point_in_polygon(poly: &[f32], x: f32, y: f32) -> bool {
    let n = poly.len() / 2;
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi, xj, yj) = (poly[2 * i], poly[2 * i + 1], poly[2 * j], poly[2 * j + 1]);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri_area(p: &[f32], t: &[u32]) -> f32 {
        let q = |i: u32| (p[2 * i as usize], p[2 * i as usize + 1]);
        cross(q(t[0]), q(t[1]), q(t[2])).abs() * 0.5
    }

    #[test]
    fn clip_square_by_concave_polygon() {
        // 정사각형 [−10, 10]² (삼각형 2개)를 L자(오목) 다각형으로 자른다.
        let sq = [-10.0, -10.0, 10.0, -10.0, 10.0, 10.0, -10.0, 10.0];
        let uv = [0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0];
        let idx = [0, 1, 2, 2, 3, 0];
        let l = [0.0, 0.0, 20.0, 0.0, 20.0, 5.0, 5.0, 5.0, 5.0, 20.0, 0.0, 20.0];
        let c = Clipper::new(&l, false, false).unwrap();
        let (p, u, i) = c.clip(&sq, &uv, &idx);
        // 안쪽 넓이: [0,10]×[0,5] + [0,5]×[5,10] = 50 + 25 = 75
        let area: f32 = i.chunks(3).map(|t| tri_area(&p, t)).sum();
        assert!((area - 75.0).abs() < 1e-3, "{area}");
        // uv: (x, y) → (x/20 + 0.5, 0.5 − y/20)
        for k in 0..p.len() / 2 {
            assert!((u[2 * k] - (p[2 * k] / 20.0 + 0.5)).abs() < 1e-4);
            assert!((u[2 * k + 1] - (0.5 - p[2 * k + 1] / 20.0)).abs() < 1e-4);
        }
        // 반전: 400 − 75 = 325
        let ci = Clipper::new(&l, false, true).unwrap();
        let (p2, _, i2) = ci.clip(&sq, &uv, &idx);
        let area2: f32 = i2.chunks(3).map(|t| tri_area(&p2, t)).sum();
        assert!((area2 - 325.0).abs() < 1e-2, "{area2}");
        // 볼록 옵션: L자의 볼록 껍질 (0,0),(20,0),(20,5),(5,20),(0,20) ∩ 정사각형 = [0,10]² − 꼭짓점 (10,10) 근처 없음 → 100
        // 껍질 변 (20,5)–(5,20): x + y = 25 이므로 [0,10]²은 전부 안쪽 → 100
        let cc = Clipper::new(&l, true, false).unwrap();
        let (p3, _, i3) = cc.clip(&sq, &uv, &idx);
        let area3: f32 = i3.chunks(3).map(|t| tri_area(&p3, t)).sum();
        assert!((area3 - 100.0).abs() < 1e-3, "{area3}");
    }
}
