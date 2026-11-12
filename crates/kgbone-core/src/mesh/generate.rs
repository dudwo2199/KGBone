//! 이미지 알파로 메시 자동 생성.
//!
//! 1. 알파 > 0 픽셀의 경계(픽셀 모서리 격자 위 변)를 이어 닫힌 고리들을 만들고, 넓이가 가장 큰 고리를 외곽선으로 쓴다.
//!    (구멍·떨어진 작은 조각은 무시한다.)
//! 2. 일직선 점을 지우고 Douglas–Peucker로 단순화한다. 허용 오차 tol = 0.75 + (1 − detail) × 1.75 픽셀.
//! 3. 단순화로 깎인 부분을 메우도록 정점을 바깥쪽(꼭짓점 법선)으로 tol/2만큼 밀고 이미지 범위로 자른다.
//! 4. detail ≥ 0.3이면 간격 max(w, h) / (3 + 9·detail)의 격자점 중 외곽선에서 간격/2 이상 떨어진 안쪽 점을 더한다.
//! 5. 외곽선을 귀 자르기로 나누고, 안쪽 점은 들어 있는 삼각형을 셋으로 쪼개 넣은 뒤 외곽선이 아닌 변을
//!    외접원 판정으로 뒤집어(들로네 방식) 모양을 고른다.
//! 정점은 이미지 중심이 원점인 로컬 좌표(y 위), uv는 0~1(v 아래).
use super::triangulate::triangulate;
use std::collections::HashMap;

/// 이미지의 알파 채널(행 우선, 위쪽 행부터).
#[derive(Clone, Debug, PartialEq)]
pub struct AlphaMask {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// 생성된 메시. 정점은 이미지 중심이 원점인 로컬 좌표(y 위로), uv는 0~1(v 아래로).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GeneratedMesh {
    pub vertices: Vec<f32>,
    pub uvs: Vec<f32>,
    pub triangles: Vec<u32>,
    /// 외곽선 정점 수(앞에서부터).
    pub hull: u32,
}

type P = (f32, f32);

fn cross(o: P, a: P, b: P) -> f32 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

fn signed_area(poly: &[P]) -> f32 {
    let n = poly.len();
    (0..n).map(|i| cross((0.0, 0.0), poly[i], poly[(i + 1) % n])).sum::<f32>() * 0.5
}

impl AlphaMask {
    fn opaque(&self, x: i64, y: i64) -> bool {
        if x < 0 || y < 0 || x >= self.width as i64 || y >= self.height as i64 {
            return false;
        }
        self.data.get((y as usize) * self.width as usize + x as usize).copied().unwrap_or(0) > 0
    }
}

/// 경계 고리들(픽셀 모서리 좌표, y 아래).
fn boundary_loops(m: &AlphaMask) -> Vec<Vec<(i64, i64)>> {
    let mut next: HashMap<(i64, i64), Vec<(i64, i64)>> = HashMap::new();
    let mut count = 0usize;
    for y in 0..m.height as i64 {
        for x in 0..m.width as i64 {
            if !m.opaque(x, y) {
                continue;
            }
            let mut add = |a: (i64, i64), b: (i64, i64)| {
                next.entry(a).or_default().push(b);
                count += 1;
            };
            if !m.opaque(x, y - 1) {
                add((x, y), (x + 1, y));
            }
            if !m.opaque(x + 1, y) {
                add((x + 1, y), (x + 1, y + 1));
            }
            if !m.opaque(x, y + 1) {
                add((x + 1, y + 1), (x, y + 1));
            }
            if !m.opaque(x - 1, y) {
                add((x, y + 1), (x, y));
            }
        }
    }
    let mut starts: Vec<(i64, i64)> = next.keys().copied().collect();
    starts.sort();
    let mut loops = vec![];
    for s in starts {
        while next.get(&s).map(|v| !v.is_empty()).unwrap_or(false) {
            let mut lp = vec![s];
            let mut cur = s;
            for _ in 0..=count {
                let Some(list) = next.get_mut(&cur) else { break };
                let Some(n) = list.pop() else { break };
                if n == s {
                    break;
                }
                lp.push(n);
                cur = n;
            }
            loops.push(lp);
        }
    }
    loops
}

fn seg_dist(p: P, a: P, b: P) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
    let (qx, qy) = (a.0 + dx * t, a.1 + dy * t);
    ((p.0 - qx).powi(2) + (p.1 - qy).powi(2)).sqrt()
}

fn douglas_peucker(pts: &[P], tol: f32, keep: &mut [bool], lo: usize, hi: usize) {
    if hi <= lo + 1 {
        return;
    }
    let (mut best, mut bi) = (0.0f32, lo);
    for i in lo + 1..hi {
        let d = seg_dist(pts[i], pts[lo], pts[hi]);
        if d > best {
            best = d;
            bi = i;
        }
    }
    if best > tol {
        keep[bi] = true;
        douglas_peucker(pts, tol, keep, lo, bi);
        douglas_peucker(pts, tol, keep, bi, hi);
    }
}

/// 닫힌 고리 단순화: 가장 먼 두 점으로 나눠 양쪽을 Douglas–Peucker.
fn simplify_closed(pts: &[P], tol: f32) -> Vec<P> {
    let n = pts.len();
    if n <= 4 {
        return pts.to_vec();
    }
    let far = (1..n).max_by(|&a, &b| {
        let d = |i: usize| (pts[i].0 - pts[0].0).powi(2) + (pts[i].1 - pts[0].1).powi(2);
        d(a).partial_cmp(&d(b)).unwrap_or(std::cmp::Ordering::Equal)
    }).unwrap_or(n / 2);
    let mut ring: Vec<P> = pts.to_vec();
    ring.push(pts[0]);
    let mut keep = vec![false; n + 1];
    keep[0] = true;
    keep[far] = true;
    keep[n] = true;
    douglas_peucker(&ring, tol, &mut keep, 0, far);
    douglas_peucker(&ring, tol, &mut keep, far, n);
    (0..n).filter(|&i| keep[i]).map(|i| ring[i]).collect()
}

fn point_in_polygon(poly: &[P], p: P) -> bool {
    let n = poly.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// 점 d가 삼각형 (a, b, c)(반시계)의 외접원 안에 있는가.
fn in_circumcircle(a: P, b: P, c: P, d: P) -> bool {
    let (ax, ay) = ((a.0 - d.0) as f64, (a.1 - d.1) as f64);
    let (bx, by) = ((b.0 - d.0) as f64, (b.1 - d.1) as f64);
    let (cx, cy) = ((c.0 - d.0) as f64, (c.1 - d.1) as f64);
    let det = (ax * ax + ay * ay) * (bx * cy - cx * by) - (bx * bx + by * by) * (ax * cy - cx * ay) + (cx * cx + cy * cy) * (ax * by - bx * ay);
    det > 1e-9
}

/// 안쪽 점 넣기 + 변 뒤집기.
fn insert_points(pts: &[P], tris: &mut Vec<[usize; 3]>, inner: std::ops::Range<usize>) {
    for pi in inner {
        let p = pts[pi];
        let found = tris.iter().position(|t| {
            let (a, b, c) = (pts[t[0]], pts[t[1]], pts[t[2]]);
            let area = cross(a, b, c);
            if area <= 0.0 {
                return false;
            }
            let (wa, wb, wc) = (cross(p, b, c) / area, cross(a, p, c) / area, cross(a, b, p) / area);
            wa > 0.02 && wb > 0.02 && wc > 0.02
        });
        if let Some(k) = found {
            let [a, b, c] = tris[k];
            tris[k] = [a, b, pi];
            tris.push([b, c, pi]);
            tris.push([c, a, pi]);
        }
    }
    for _ in 0..64 {
        let mut edges: HashMap<(usize, usize), usize> = HashMap::new();
        for (k, t) in tris.iter().enumerate() {
            for e in 0..3 {
                edges.insert((t[e], t[(e + 1) % 3]), k);
            }
        }
        let mut flipped = false;
        let mut used = vec![false; tris.len()];
        for k in 0..tris.len() {
            if used[k] {
                continue;
            }
            for e in 0..3 {
                let t = tris[k];
                let (a, b, c) = (t[e], t[(e + 1) % 3], t[(e + 2) % 3]);
                let Some(&k2) = edges.get(&(b, a)) else { continue };
                if used[k2] || k2 == k {
                    continue;
                }
                let t2 = tris[k2];
                let Some(&d) = t2.iter().find(|&&v| v != a && v != b) else { continue };
                if !in_circumcircle(pts[a], pts[b], pts[c], pts[d]) {
                    continue;
                }
                let (n1, n2) = ([a, d, c], [d, b, c]);
                if cross(pts[n1[0]], pts[n1[1]], pts[n1[2]]) <= 1e-6 || cross(pts[n2[0]], pts[n2[1]], pts[n2[2]]) <= 1e-6 {
                    continue;
                }
                tris[k] = n1;
                tris[k2] = n2;
                used[k] = true;
                used[k2] = true;
                flipped = true;
                break;
            }
        }
        if !flipped {
            break;
        }
    }
}

/// `detail`(0~1): 클수록 외곽선 정점이 많고 내부 정점을 더 넣는다.
pub fn generate_from_alpha(mask: &AlphaMask, detail: f32) -> GeneratedMesh {
    let detail = detail.clamp(0.0, 1.0);
    let (w, h) = (mask.width as f32, mask.height as f32);
    let loops = boundary_loops(mask);
    let Some(outline) = loops
        .into_iter()
        .map(|l| l.into_iter().map(|(x, y)| (x as f32, y as f32)).collect::<Vec<P>>())
        .max_by(|a, b| signed_area(a).abs().partial_cmp(&signed_area(b).abs()).unwrap_or(std::cmp::Ordering::Equal))
    else {
        return GeneratedMesh::default();
    };
    // 일직선 점 제거
    let n = outline.len();
    let corners: Vec<P> = (0..n).filter(|&i| cross(outline[(i + n - 1) % n], outline[i], outline[(i + 1) % n]) != 0.0).map(|i| outline[i]).collect();
    let tol = 0.75 + (1.0 - detail) * 1.75;
    let mut hull = simplify_closed(&corners, tol);
    if hull.len() < 3 {
        return GeneratedMesh::default();
    }
    // 바깥으로 밀기
    let ccw = signed_area(&hull) > 0.0; // y 아래 좌표에서의 부호
    let m = hull.len();
    let normal = |a: P, b: P| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = (dx * dx + dy * dy).sqrt().max(1e-9);
        // 내부가 왼쪽(ccw)이면 바깥 = 오른쪽 법선
        if ccw {
            (dy / l, -dx / l)
        } else {
            (-dy / l, dx / l)
        }
    };
    let pushed: Vec<P> = (0..m)
        .map(|i| {
            let (p0, p1, p2) = (hull[(i + m - 1) % m], hull[i], hull[(i + 1) % m]);
            let (n1, n2) = (normal(p0, p1), normal(p1, p2));
            let (nx, ny) = (n1.0 + n2.0, n1.1 + n2.1);
            let l = (nx * nx + ny * ny).sqrt();
            let off = tol * 0.5;
            let (ox, oy) = if l > 1e-6 { (nx / l * off, ny / l * off) } else { (0.0, 0.0) };
            ((p1.0 + ox).clamp(0.0, w), (p1.1 + oy).clamp(0.0, h))
        })
        .collect();
    hull = pushed;
    let mut pts = hull.clone();
    let hull_n = hull.len();
    // 안쪽 점
    if detail >= 0.3 {
        let s = w.max(h) / (3.0 + 9.0 * detail);
        let mut y = s * 0.5;
        while y < h {
            let mut x = s * 0.5;
            while x < w {
                let p = (x, y);
                if point_in_polygon(&hull, p) && (0..hull_n).all(|i| seg_dist(p, hull[i], hull[(i + 1) % hull_n]) > s * 0.5) {
                    pts.push(p);
                }
                x += s;
            }
            y += s;
        }
    }
    let flat: Vec<f32> = hull.iter().flat_map(|p| [p.0, p.1]).collect();
    // 삼각분할은 y 아래 좌표에서 하지만 결과 방향은 y 위 좌표에서 반시계가 되도록 나중에 뒤집는다.
    let mut tris: Vec<[usize; 3]> = triangulate(&flat).chunks(3).map(|t| [t[0] as usize, t[1] as usize, t[2] as usize]).collect();
    insert_points(&pts, &mut tris, hull_n..pts.len());
    let mut out = GeneratedMesh { hull: hull_n as u32, ..Default::default() };
    for p in &pts {
        out.vertices.push(p.0 - w * 0.5);
        out.vertices.push(h * 0.5 - p.1);
        out.uvs.push(if w > 0.0 { p.0 / w } else { 0.0 });
        out.uvs.push(if h > 0.0 { p.1 / h } else { 0.0 });
    }
    for t in tris {
        if cross(pts[t[0]], pts[t[1]], pts[t[2]]).abs() > 1e-6 {
            // y를 뒤집으면 방향이 바뀌므로 순서를 바꿔 y 위 좌표에서 반시계로.
            out.triangles.extend_from_slice(&[t[0] as u32, t[2] as u32, t[1] as u32]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri_area(v: &[f32], t: &[u32]) -> f32 {
        let p = |i: u32| (v[2 * i as usize], v[2 * i as usize + 1]);
        cross(p(t[0]), p(t[1]), p(t[2])) * 0.5
    }

    /// 불투명 픽셀 중심이 메시 삼각형 안에 드는 비율.
    fn coverage(mask: &AlphaMask, g: &GeneratedMesh) -> f32 {
        let (w, h) = (mask.width as f32, mask.height as f32);
        let (mut total, mut hit) = (0, 0);
        for y in 0..mask.height {
            for x in 0..mask.width {
                if mask.data[(y * mask.width + x) as usize] == 0 {
                    continue;
                }
                total += 1;
                let p = (x as f32 + 0.5 - w * 0.5, h * 0.5 - (y as f32 + 0.5));
                let inside = g.triangles.chunks(3).any(|t| {
                    let q = |i: u32| (g.vertices[2 * i as usize], g.vertices[2 * i as usize + 1]);
                    let (a, b, c) = (q(t[0]), q(t[1]), q(t[2]));
                    cross(a, b, p) >= -1e-4 && cross(b, c, p) >= -1e-4 && cross(c, a, p) >= -1e-4
                });
                if inside {
                    hit += 1;
                }
            }
        }
        hit as f32 / total as f32
    }

    fn circle(size: u32, r: f32) -> AlphaMask {
        let c = size as f32 / 2.0;
        let data = (0..size * size)
            .map(|i| {
                let (x, y) = ((i % size) as f32 + 0.5 - c, (i / size) as f32 + 0.5 - c);
                if x * x + y * y <= r * r { 255 } else { 0 }
            })
            .collect();
        AlphaMask { width: size, height: size, data }
    }

    fn rect(w: u32, h: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> AlphaMask {
        let data = (0..w * h).map(|i| if (x0..x1).contains(&(i % w)) && (y0..y1).contains(&(i / w)) { 200 } else { 0 }).collect();
        AlphaMask { width: w, height: h, data }
    }

    #[test]
    fn f_d03_generate_from_alpha() {
        for (mask, name) in [(circle(100, 40.0), "원"), (rect(64, 48, 8, 4, 56, 40), "사각")] {
            for detail in [0.0, 0.5, 1.0] {
                let g = generate_from_alpha(&mask, detail);
                assert!(g.triangles.len() >= 3, "{name}");
                assert_eq!(g.vertices.len(), g.uvs.len());
                for t in g.triangles.chunks(3) {
                    assert!(tri_area(&g.vertices, t) > 0.0, "{name} 넓이 0 이하 삼각형");
                }
                for uv in &g.uvs {
                    assert!((0.0..=1.0).contains(uv));
                }
                let cov = coverage(&mask, &g);
                assert!(cov >= 0.95, "{name} detail {detail}: 덮은 비율 {cov}");
                if detail >= 0.5 && name == "원" {
                    assert!(g.vertices.len() / 2 > g.hull as usize, "안쪽 점이 있어야 함");
                }
            }
        }
        // 사각형은 외곽선 4점(밀린 모서리)으로 줄어든다
        let g = generate_from_alpha(&rect(64, 48, 8, 4, 56, 40), 0.0);
        assert_eq!(g.hull, 4);
        assert!(generate_from_alpha(&AlphaMask { width: 4, height: 4, data: vec![0; 16] }, 0.5).triangles.is_empty());
    }
}
