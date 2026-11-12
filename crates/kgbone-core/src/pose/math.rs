//! 제약 계산이 함께 쓰는 작은 수학 도우미.
use super::WorldBone;

/// 각도(도)를 (−180, 180] 범위로.
pub(crate) fn wrap_deg(a: f32) -> f32 {
    let mut r = a % 360.0;
    if r > 180.0 {
        r -= 360.0;
    } else if r <= -180.0 {
        r += 360.0;
    }
    r
}

/// 월드 행렬 전체(두 축)를 `deg`도 돌린다(위치는 그대로).
pub(crate) fn rotate_world(w: &mut WorldBone, deg: f32) {
    let (s, c) = deg.to_radians().sin_cos();
    let (a, b, cc, d) = (w.a, w.b, w.c, w.d);
    w.a = c * a - s * cc;
    w.c = s * a + c * cc;
    w.b = c * b - s * d;
    w.d = s * b + c * d;
}

/// 월드 X축(첫 열)만 `deg`도 돌린다(기울기 X).
pub(crate) fn rotate_world_x_axis(w: &mut WorldBone, deg: f32) {
    let (s, c) = deg.to_radians().sin_cos();
    let (a, cc) = (w.a, w.c);
    w.a = c * a - s * cc;
    w.c = s * a + c * cc;
}

/// 월드 기울기 Y: 두 축 사이 각 − 90(도). 반사가 있으면 ±180 근처가 된다.
pub(crate) fn world_shear_y(w: &WorldBone) -> f32 {
    wrap_deg(w.d.atan2(w.b).to_degrees() - w.c.atan2(w.a).to_degrees() - 90.0)
}

/// 월드 값(회전, 크기 X·Y, 기울기 Y)으로 행렬을 만든다(위치는 따로).
pub(crate) fn world_from_parts(rotation: f32, scale_x: f32, scale_y: f32, shear_y: f32, x: f32, y: f32) -> WorldBone {
    let rx = rotation.to_radians();
    let ry = (rotation + 90.0 + shear_y).to_radians();
    WorldBone { a: rx.cos() * scale_x, c: rx.sin() * scale_x, b: ry.cos() * scale_y, d: ry.sin() * scale_y, x, y }
}

/// 부모 공간: 자식 본 하나가 놓이는 좌표계(부모의 상속 행렬 + 부모 원점). 부모가 없으면 월드.
pub(crate) fn parent_space(parent: Option<&WorldBone>, inherit: crate::model::Inherit) -> WorldBone {
    match parent {
        None => WorldBone::default(),
        Some(p) => {
            let m = super::bone_transform::inherited_matrix(p, inherit);
            WorldBone { a: m[0], b: m[1], c: m[2], d: m[3], x: p.x, y: p.y }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_and_parts() {
        assert_eq!(wrap_deg(190.0), -170.0);
        assert_eq!(wrap_deg(-190.0), 170.0);
        assert_eq!(wrap_deg(180.0), 180.0);
        let w = world_from_parts(30.0, 2.0, 0.5, 10.0, 0.0, 0.0);
        assert!((w.rotation_x() - 30.0).abs() < 1e-4);
        assert!((w.scale_x() - 2.0).abs() < 1e-5 && (w.scale_y() - 0.5).abs() < 1e-5);
        assert!((world_shear_y(&w) - 10.0).abs() < 1e-3);
    }
}
