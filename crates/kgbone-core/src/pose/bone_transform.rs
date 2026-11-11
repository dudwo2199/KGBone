//! 본 하나의 월드 변환 계산(상속 종류별).

/// 월드 변환: 2x2 행렬 [a b; c d]와 위치(x, y). 로컬 점 (lx, ly) → (a·lx + b·ly + x, c·lx + d·ly + y).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldBone {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub x: f32,
    pub y: f32,
}

impl Default for WorldBone {
    fn default() -> Self {
        WorldBone { a: 1.0, b: 0.0, c: 0.0, d: 1.0, x: 0.0, y: 0.0 }
    }
}

impl WorldBone {
    pub fn local_to_world(&self, lx: f32, ly: f32) -> (f32, f32) {
        (self.a * lx + self.b * ly + self.x, self.c * lx + self.d * ly + self.y)
    }
    pub fn world_to_local(&self, wx: f32, wy: f32) -> (f32, f32) {
        let det = self.a * self.d - self.b * self.c;
        let inv = if det.abs() < 1e-12 { 0.0 } else { 1.0 / det };
        let (dx, dy) = (wx - self.x, wy - self.y);
        ((dx * self.d - dy * self.b) * inv, (dy * self.a - dx * self.c) * inv)
    }
    /// 월드 X축 방향 각도(도).
    pub fn rotation_x(&self) -> f32 {
        self.c.atan2(self.a).to_degrees()
    }
    pub fn scale_x(&self) -> f32 {
        (self.a * self.a + self.c * self.c).sqrt()
    }
    pub fn scale_y(&self) -> f32 {
        (self.b * self.b + self.d * self.d).sqrt()
    }
}

use crate::model::{Inherit, Skeleton, Transform};

crate::json_object!(WorldBone { a: "a" = 1.0, b: "b" = 0.0, c: "c" = 0.0, d: "d" = 1.0, x: "x" = 0.0, y: "y" = 0.0 });

/// 로컬 값 하나의 행렬(부모 없음).
pub(crate) fn local_matrix(t: &Transform) -> [f32; 4] {
    let rx = (t.rotation + t.shear_x).to_radians();
    let ry = (t.rotation + 90.0 + t.shear_y).to_radians();
    [rx.cos() * t.scale_x, ry.cos() * t.scale_y, rx.sin() * t.scale_x, ry.sin() * t.scale_y]
}

fn mul(p: [f32; 4], l: [f32; 4]) -> [f32; 4] {
    [p[0] * l[0] + p[1] * l[2], p[0] * l[1] + p[1] * l[3], p[2] * l[0] + p[3] * l[2], p[2] * l[1] + p[3] * l[3]]
}

/// 상속 종류에 따라 자식이 물려받을 부모 행렬.
/// - normal: 부모 행렬 전체
/// - onlyTranslation: 단위 행렬(위치만 물려받음)
/// - noRotationOrReflection: 부모 행렬에서 회전 R(θ)와 반사를 뺀 것(크기·기울기만)
/// - noScale: 부모의 회전(반사 포함)만
/// - noScaleOrReflection: 부모의 회전만
/// 여기서 θ = atan2(c, a) (부모 X축 방향).
pub(crate) fn inherited_matrix(parent: &WorldBone, inherit: Inherit) -> [f32; 4] {
    let p = [parent.a, parent.b, parent.c, parent.d];
    let theta = parent.c.atan2(parent.a);
    let (cs, sn) = (theta.cos(), theta.sin());
    let reflect = parent.a * parent.d - parent.b * parent.c < 0.0;
    match inherit {
        Inherit::Normal => p,
        Inherit::OnlyTranslation => [1.0, 0.0, 0.0, 1.0],
        Inherit::NoRotationOrReflection => {
            // R(-θ)·P
            let m = mul([cs, sn, -sn, cs], p);
            if reflect {
                [m[0], -m[1], m[2], -m[3]]
            } else {
                m
            }
        }
        Inherit::NoScale => {
            if reflect {
                [cs, sn, sn, -cs]
            } else {
                [cs, -sn, sn, cs]
            }
        }
        Inherit::NoScaleOrReflection => [cs, -sn, sn, cs],
    }
}

/// 본 하나의 월드 변환.
pub(crate) fn world_of(local: &Transform, inherit: Inherit, parent: Option<&WorldBone>) -> WorldBone {
    let l = local_matrix(local);
    match parent {
        None => WorldBone { a: l[0], b: l[1], c: l[2], d: l[3], x: local.x, y: local.y },
        Some(p) => {
            let m = mul(inherited_matrix(p, inherit), l);
            let (x, y) = p.local_to_world(local.x, local.y);
            WorldBone { a: m[0], b: m[1], c: m[2], d: m[3], x, y }
        }
    }
}

/// 모든 본의 월드 변환(제약 없이). `locals`·`inherits`는 `skel.bones` 순서. 부모가 앞에 있다고 가정한다.
pub fn compute_world(skel: &Skeleton, locals: &[Transform], inherits: &[Inherit]) -> Vec<WorldBone> {
    let mut out: Vec<WorldBone> = Vec::with_capacity(skel.bones.len());
    for (i, b) in skel.bones.iter().enumerate() {
        let parent = b.parent.as_deref().and_then(|n| skel.bone_index(n)).filter(|&pi| pi < i).map(|pi| out[pi]);
        out.push(world_of(&locals[i], inherits[i], parent.as_ref()));
    }
    out
}

/// 월드 변환 `w`를 부모 `parent` 아래 로컬 값으로 되돌린다(상속 normal 기준, 기울기는 shearY로 표현).
pub fn world_to_local_transform(w: &WorldBone, parent: Option<&WorldBone>, inherit: Inherit) -> Transform {
    let (pm, (x, y)) = match parent {
        None => ([1.0, 0.0, 0.0, 1.0], (w.x, w.y)),
        Some(p) => (inherited_matrix(p, inherit), p.world_to_local(w.x, w.y)),
    };
    // L = Pm⁻¹ · W
    let det = pm[0] * pm[3] - pm[1] * pm[2];
    let inv = if det.abs() < 1e-12 { 0.0 } else { 1.0 / det };
    let pi = [pm[3] * inv, -pm[1] * inv, -pm[2] * inv, pm[0] * inv];
    let l = mul(pi, [w.a, w.b, w.c, w.d]);
    // local_matrix의 정확한 역: 1열 = scaleX·(cos r, sin r), 2열 = scaleY·(cos(r+90+shearY), sin(…))
    let sx = (l[0] * l[0] + l[2] * l[2]).sqrt();
    let rotation = l[2].atan2(l[0]).to_degrees();
    let sy = (l[1] * l[1] + l[3] * l[3]).sqrt();
    let ry = l[3].atan2(l[1]).to_degrees();
    let mut shear_y = ry - 90.0 - rotation;
    while shear_y > 180.0 {
        shear_y -= 360.0;
    }
    while shear_y < -180.0 {
        shear_y += 360.0;
    }
    Transform { x, y, rotation, scale_x: sx, scale_y: sy, shear_x: 0.0, shear_y: if shear_y.abs() < 1e-4 { 0.0 } else { shear_y } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Bone;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn f_b04_parent_child_world_position() {
        // 부모: (10, 0) 회전 90°. 자식: 로컬 (5, 0) → 월드 (10, 5)
        let mut skel = Skeleton::new_with_root();
        let mut p = Bone::new("p", Some("root".into()));
        p.x = 10.0;
        p.rotation = 90.0;
        let mut c = Bone::new("c", Some("p".into()));
        c.x = 5.0;
        skel.bones.push(p);
        skel.bones.push(c);
        let locals: Vec<_> = skel.bones.iter().map(|b| b.local()).collect();
        let inh: Vec<_> = skel.bones.iter().map(|b| b.inherit).collect();
        let w = compute_world(&skel, &locals, &inh);
        assert!(close(w[2].x, 10.0) && close(w[2].y, 5.0), "{:?}", w[2]);
        assert!(close(w[2].rotation_x(), 90.0));
    }

    #[test]
    fn world_local_roundtrip() {
        let parent = world_of(&Transform { x: 3.0, y: 4.0, rotation: 30.0, scale_x: 2.0, scale_y: 0.5, ..Default::default() }, Inherit::Normal, None);
        let local = Transform { x: 7.0, y: -2.0, rotation: 15.0, scale_x: 1.5, scale_y: 1.2, ..Default::default() };
        let w = world_of(&local, Inherit::Normal, Some(&parent));
        let back = world_to_local_transform(&w, Some(&parent), Inherit::Normal);
        let w2 = world_of(&back, Inherit::Normal, Some(&parent));
        for (a, b) in [(w.a, w2.a), (w.b, w2.b), (w.c, w2.c), (w.d, w2.d), (w.x, w2.x), (w.y, w2.y)] {
            assert!(close(a, b), "{w:?} {w2:?}");
        }
    }
}
