//! 그리기 목록과 에디터 표시 도형 만들기(첨부물 종류별), 클리핑 적용.
//!
//! - region: 슬롯 본 기준 (x, y, rotation, scaleX, scaleY)로 놓인 width × height 사각형. 네 모서리 순서는
//!   왼아래, 오른아래, 오른위, 왼위(로컬), uv (0,1)(1,1)(1,0)(0,0), 삼각형 [0,1,2, 2,3,0].
//! - mesh: 가중치/비가중치 정점 + deform. linkedmesh: 부모 메시의 uv·삼각형·정점(timelines면 부모 deform).
//! - clipping: 그 슬롯부터 end 슬롯(포함)까지 그려지는 것을 자른다. 클리핑 슬롯 자체는 그리지 않는다.
//! - 비활성 본에 붙은 슬롯은 그리지 않는다.
use super::clipping::Clipper;
use super::mesh_deform::{find_attachment, mesh_world_vertices, resolve_mesh, vertex_world_positions};
use super::{DrawItem, Overlay, SkeletonPose, WorldBone};
use crate::model::{Attachment, Color, RegionAttachment, Sequence, Skeleton};

pub(crate) struct Built {
    pub draw: Vec<DrawItem>,
    pub overlays: Vec<Overlay>,
}

fn mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2], a[3] * b[3]]
}

/// region의 네 모서리 월드 좌표.
pub(crate) fn region_corners(r: &RegionAttachment, bone: &WorldBone) -> Vec<f32> {
    let (hw, hh) = (r.width * 0.5, r.height * 0.5);
    let (s, c) = r.rotation.to_radians().sin_cos();
    let mut out = Vec::with_capacity(8);
    for (lx, ly) in [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)] {
        let (sx, sy) = (lx * r.scale_x, ly * r.scale_y);
        let (x, y) = (r.x + c * sx - s * sy, r.y + s * sx + c * sy);
        let (wx, wy) = bone.local_to_world(x, y);
        out.push(wx);
        out.push(wy);
    }
    out
}

/// 시퀀스 프레임 이미지 경로: 경로 + (start + 프레임)을 digits 자리로.
fn sequence_path(base: &str, seq: Option<&Sequence>, frame: Option<i32>) -> String {
    match seq {
        None => base.to_string(),
        Some(s) => {
            let f = frame.unwrap_or(s.setup as i32).max(0) as u32 + s.start;
            format!("{base}{:0width$}", f, width = s.digits as usize)
        }
    }
}

pub(crate) fn build(skel: &Skeleton, pose: &SkeletonPose, skin: Option<&str>) -> Built {
    let mut draw = vec![];
    let mut overlays = vec![];
    let mut clip: Option<(Clipper, Option<String>)> = None;
    for &si in &pose.draw_order {
        let Some(slot) = skel.slots.get(si) else { continue };
        let bi = skel.bone_index(&slot.bone);
        let active = bi.map(|i| pose.active[i]).unwrap_or(false);
        let bone = bi.map(|i| pose.world[i]).unwrap_or_default();
        let found = if active {
            pose.slot_attachments[si].as_deref().and_then(|n| find_attachment(skel, skin, &slot.name, n).map(|(_, a)| (n, a)))
        } else {
            None
        };
        if let Some((name, att)) = found {
            let deform = pose.deform.get(&(slot.name.clone(), name.to_string())).map(Vec::as_slice);
            let overlay = |kind: &str, points: Vec<f32>, closed: bool, color: Color| Overlay {
                kind: kind.into(),
                slot: slot.name.clone(),
                attachment: name.into(),
                points,
                indices: vec![],
                closed,
                color: color.to_f32(),
            };
            let slot_color = pose.slot_colors[si];
            let frame = pose.sequence_index.get(&(slot.name.clone(), name.to_string())).copied();
            let mut item: Option<DrawItem> = None;
            match att {
                Attachment::Region(r) => {
                    item = Some(DrawItem {
                        slot: slot.name.clone(),
                        attachment: name.into(),
                        image: sequence_path(att.image_path(name).unwrap_or(name), r.sequence.as_ref(), frame),
                        blend: slot.blend,
                        color: mul(slot_color, r.color.to_f32()),
                        dark: pose.slot_darks[si],
                        positions: region_corners(r, &bone),
                        uvs: vec![0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0],
                        indices: vec![0, 1, 2, 2, 3, 0],
                    });
                }
                Attachment::Mesh(_) | Attachment::LinkedMesh(_) => {
                    if let Some((mesh, deform_name)) = resolve_mesh(skel, &slot.name, name, att) {
                        let positions = mesh_world_vertices(skel, pose, si, mesh, deform_name);
                        let (color, seq) = match att {
                            Attachment::LinkedMesh(l) => (l.color, l.sequence.as_ref()),
                            _ => (mesh.color, mesh.sequence.as_ref()),
                        };
                        let mut ov = overlay("mesh", positions.clone(), true, color);
                        ov.indices = mesh.triangles.clone();
                        overlays.push(ov);
                        item = Some(DrawItem {
                            slot: slot.name.clone(),
                            attachment: name.into(),
                            image: sequence_path(att.image_path(name).unwrap_or(name), seq, frame),
                            blend: slot.blend,
                            color: mul(slot_color, color.to_f32()),
                            dark: pose.slot_darks[si],
                            positions,
                            uvs: mesh.uvs.clone(),
                            indices: mesh.triangles.clone(),
                        });
                    }
                }
                Attachment::BoundingBox(b) => {
                    overlays.push(overlay("boundingbox", vertex_world_positions(b.weighted, &b.vertices, &bone, &pose.world, deform), true, b.color));
                }
                Attachment::Path(p) => {
                    overlays.push(overlay("path", vertex_world_positions(p.weighted, &p.vertices, &bone, &pose.world, deform), p.closed, p.color));
                }
                Attachment::Point(p) => {
                    let (x, y) = bone.local_to_world(p.x, p.y);
                    let (s, c) = p.rotation.to_radians().sin_cos();
                    let (dx, dy) = (bone.a * c + bone.b * s, bone.c * c + bone.d * s);
                    let len = (dx * dx + dy * dy).sqrt().max(1e-9);
                    overlays.push(overlay("point", vec![x, y, dx / len, dy / len], false, p.color));
                }
                Attachment::Clipping(c) => {
                    let poly = vertex_world_positions(c.weighted, &c.vertices, &bone, &pose.world, deform);
                    overlays.push(overlay("clipping", poly.clone(), true, c.color));
                    clip = Clipper::new(&poly, c.convex, c.inverse).map(|cl| (cl, c.end.clone()));
                    continue;
                }
            }
            if let Some(mut it) = item {
                if let Some((cl, _)) = &clip {
                    let (p, u, i) = cl.clip(&it.positions, &it.uvs, &it.indices);
                    it.positions = p;
                    it.uvs = u;
                    it.indices = i;
                }
                if !it.indices.is_empty() {
                    draw.push(it);
                }
            }
        }
        if let Some((_, end)) = &clip {
            if end.as_deref() == Some(slot.name.as_str()) {
                clip = None;
            }
        }
    }
    Built { draw, overlays }
}

#[cfg(test)]
mod tests {
    use crate::model::{Attachment, Bone, ClippingAttachment, Color, PointAttachment, RegionAttachment, Skeleton, Skin, Slot};
    use crate::pose::clipping::point_in_polygon;
    use crate::pose::{PoseEngine, PoseRequest};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    fn base() -> Skeleton {
        let mut s = Skeleton::new_with_root();
        // 본: (10, 20), 회전 90°
        let mut b = Bone::new("b", Some("root".into()));
        b.x = 10.0;
        b.y = 20.0;
        b.rotation = 90.0;
        s.bones.push(b);
        s
    }

    #[test]
    fn f_d01_region_corners_world() {
        // region: 본 기준 (5, 0), 회전 0, scaleX 2, 폭 10 높이 4
        // 로컬 모서리: (−5·2, −2), (5·2, −2), (10, 2), (−10, 2) + (5, 0) → (−5,−2), (15,−2), (15,2), (−5,2)
        // 본(회전 90°, (10,20)): (x, y) → (10 − y, 20 + x)
        // → (12, 15), (12, 35), (8, 35), (8, 15)
        let mut s = base();
        s.slots.push(Slot { attachment: Some("img".into()), ..Slot::new("s", "b") });
        let r = RegionAttachment { x: 5.0, scale_x: 2.0, width: 10.0, height: 4.0, ..Default::default() };
        s.skins[0].attachments.entry("s").insert("img", Attachment::Region(r));
        let out = PoseEngine::new().compute(&s, &PoseRequest::default());
        let d = &out.draw[0];
        let want = [12.0, 15.0, 12.0, 35.0, 8.0, 35.0, 8.0, 15.0];
        for (a, b) in d.positions.iter().zip(want) {
            assert!(close(*a, b), "{:?}", d.positions);
        }
        assert_eq!(d.uvs, vec![0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0]);
        assert_eq!(d.indices, vec![0, 1, 2, 2, 3, 0]);
        assert_eq!(d.image, "img");
    }

    #[test]
    fn f_d10_point_world() {
        // 포인트: 본 기준 (3, 4), 회전 30° → 월드 (10 − 4, 20 + 3) = (6, 23), 방향 30 + 90 = 120°
        let mut s = base();
        s.slots.push(Slot { attachment: Some("pt".into()), ..Slot::new("s", "b") });
        s.skins[0].attachments.entry("s").insert("pt", Attachment::Point(PointAttachment { x: 3.0, y: 4.0, rotation: 30.0, color: Color::POINT }));
        let out = PoseEngine::new().compute(&s, &PoseRequest::default());
        let o = &out.overlays[0];
        assert_eq!(o.kind, "point");
        assert!(close(o.points[0], 6.0) && close(o.points[1], 23.0));
        assert!(close(o.points[3].atan2(o.points[2]).to_degrees(), 120.0));
    }

    fn clip_skel(inverse: bool) -> Skeleton {
        let mut s = Skeleton::new_with_root();
        s.slots.push(Slot { attachment: Some("c".into()), ..Slot::new("clip", "root") });
        s.slots.push(Slot { attachment: Some("img".into()), ..Slot::new("s", "root") });
        s.slots.push(Slot { attachment: Some("img".into()), ..Slot::new("after", "root") });
        let tri = vec![-5.0, -5.0, 30.0, -5.0, -5.0, 30.0];
        let c = ClippingAttachment { end: Some("s".into()), vertex_count: 3, weighted: false, vertices: tri, inverse, convex: false, color: Color::CLIPPING };
        let skin: &mut Skin = &mut s.skins[0];
        skin.attachments.entry("clip").insert("c", Attachment::Clipping(c));
        skin.attachments.entry("s").insert("img", Attachment::Region(RegionAttachment { width: 20.0, height: 20.0, ..Default::default() }));
        skin.attachments.entry("after").insert("img", Attachment::Region(RegionAttachment { width: 20.0, height: 20.0, ..Default::default() }));
        s
    }

    #[test]
    fn f_d11_clipping_inside_and_inverse() {
        let poly = [-5.0, -5.0, 30.0, -5.0, -5.0, 30.0];
        let out = PoseEngine::new().compute(&clip_skel(false), &PoseRequest::default());
        assert_eq!(out.draw.len(), 2);
        let d = &out.draw[0];
        assert!(!d.indices.is_empty());
        // 모든 결과 정점이 클립 삼각형 안(경계 포함, 약간 안쪽으로 당겨 검사)
        for p in d.positions.chunks(2) {
            let (cx, cy) = (5.0, 5.0);
            let (x, y) = (p[0] + (cx - p[0]) * 1e-3, p[1] + (cy - p[1]) * 1e-3);
            assert!(point_in_polygon(&poly, x, y), "{p:?}");
        }
        // end 다음 슬롯은 잘리지 않는다(정점 4개)
        assert_eq!(out.draw[1].positions.len(), 8);
        // 반전: 결과 삼각형의 무게중심은 모두 클립 다각형 밖
        let out = PoseEngine::new().compute(&clip_skel(true), &PoseRequest::default());
        let d = &out.draw[0];
        assert!(!d.indices.is_empty());
        for t in d.indices.chunks(3) {
            let (mut x, mut y) = (0.0, 0.0);
            for &i in t {
                x += d.positions[2 * i as usize] / 3.0;
                y += d.positions[2 * i as usize + 1] / 3.0;
            }
            assert!(!point_in_polygon(&poly, x, y), "{x} {y}");
        }
    }
}
