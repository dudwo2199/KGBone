//! 엔진 전체 흐름 테스트: 상속, 스킨 전용 본, 가중치 메시, 타임라인 값.
use super::{compute_world, PoseEngine, PoseRequest, SkeletonPose};
use crate::anim::{apply_animation, ApplyOptions};
use crate::model::*;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

fn req_at(anim: &str, t: f32) -> PoseRequest {
    PoseRequest { animation: Some(anim.into()), time: t, ..Default::default() }
}

#[test]
fn f_b05_inherit_modes() {
    // 부모: 회전 90°, 크기 (2, 2) → 행렬 [a b; c d] = [0 −2; 2 0]. 자식: 로컬 (10, 0), 회전 0.
    // 위치는 언제나 부모 전체로 옮긴다: (0·10, 2·10) = (0, 20).
    // - normal: 자식 행렬 = 부모 → 회전 90°, 크기 2
    // - onlyTranslation: 단위 → 회전 0°, 크기 1
    // - noRotationOrReflection: R(−90°)·P = [0 1; −1 0]·[0 −2; 2 0] = [2 0; 0 2] → 회전 0°, 크기 2
    // - noScale / noScaleOrReflection: 회전 90°만 → 크기 1
    let expect = [
        (Inherit::Normal, 90.0, 2.0),
        (Inherit::OnlyTranslation, 0.0, 1.0),
        (Inherit::NoRotationOrReflection, 0.0, 2.0),
        (Inherit::NoScale, 90.0, 1.0),
        (Inherit::NoScaleOrReflection, 90.0, 1.0),
    ];
    for (inh, rot, scale) in expect {
        let mut s = Skeleton::new_with_root();
        let mut p = Bone::new("p", Some("root".into()));
        p.rotation = 90.0;
        p.scale_x = 2.0;
        p.scale_y = 2.0;
        let mut c = Bone::new("c", Some("p".into()));
        c.x = 10.0;
        c.inherit = inh;
        s.bones.push(p);
        s.bones.push(c);
        let w = super::setup_world(&s);
        assert!(close(w[2].x, 0.0) && close(w[2].y, 20.0), "{inh:?} {:?}", w[2]);
        assert!(close(w[2].rotation_x(), rot) && close(w[2].scale_x(), scale) && close(w[2].scale_y(), scale), "{inh:?} {:?}", w[2]);
    }
    // 반사된 부모(scaleY = −2): P = [0 2; 2 0] (det < 0), θ = 90°.
    // noScale은 반사를 유지: [cos −sin·(−1)…] = [0 1; 1 0] → 자식 Y축 = (1, 0)
    // noScaleOrReflection은 회전만: [0 −1; 1 0] → 자식 Y축 = (−1, 0)
    for (inh, by) in [(Inherit::NoScale, 1.0), (Inherit::NoScaleOrReflection, -1.0)] {
        let mut s = Skeleton::new_with_root();
        let mut p = Bone::new("p", Some("root".into()));
        p.rotation = 90.0;
        p.scale_x = 2.0;
        p.scale_y = -2.0;
        let mut c = Bone::new("c", Some("p".into()));
        c.inherit = inh;
        s.bones.push(p);
        s.bones.push(c);
        let w = super::setup_world(&s);
        assert!(close(w[2].b, by) && close(w[2].d, 0.0), "{inh:?} {:?}", w[2]);
    }
}

#[test]
fn f_b04_animated_transform_world() {
    // 부모 (10, 0) 회전 90°, 자식 로컬 (5, 0) 회전 30°, scaleX 2 → 월드 위치 (10, 5), 회전 120°, X축 길이 2
    let mut s = Skeleton::new_with_root();
    let mut p = Bone::new("p", Some("root".into()));
    p.x = 10.0;
    p.rotation = 90.0;
    let mut c = Bone::new("c", Some("p".into()));
    c.x = 5.0;
    c.rotation = 30.0;
    c.scale_x = 2.0;
    c.shear_y = 10.0;
    s.bones.push(p);
    s.bones.push(c);
    let out = PoseEngine::new().compute(&s, &PoseRequest::default());
    let w = out.bones[2];
    assert!(close(w.x, 10.0) && close(w.y, 5.0) && close(w.rotation_x(), 120.0) && close(w.scale_x(), 2.0));
    // shearY 10 → Y축 각 = 120 + 90 + 10 = 220°
    assert!(close(w.d.atan2(w.b).to_degrees(), -140.0));
}

#[test]
fn f_e03_skin_required_bones() {
    let mut s = Skeleton::new_with_root();
    let mut hat = Bone::new("hat", Some("root".into()));
    hat.skin_required = true;
    s.bones.push(hat);
    s.bones.push(Bone::new("feather", Some("hat".into())));
    let mut skin = Skin::new("s1");
    skin.bones.push("hat".into());
    s.skins.push(skin);
    s.slots.push(Slot { attachment: Some("img".into()), ..Slot::new("hatSlot", "hat") });
    s.skins[0].attachments.entry("hatSlot").insert("img", Attachment::Region(RegionAttachment::default()));
    let off = PoseEngine::new().compute(&s, &PoseRequest::default());
    assert_eq!(off.active, vec![true, false, false]);
    assert!(off.draw.is_empty());
    let on = PoseEngine::new().compute(&s, &PoseRequest { skin: Some("s1".into()), ..Default::default() });
    assert_eq!(on.active, vec![true, true, true]);
    assert_eq!(on.draw.len(), 1);
}

/// samples/spine43/mesh-weights.json과 같은 구조: root, tail1(길이 64), tail2(tail1 기준 x 64).
fn weighted_skel(tail2_rot: f32) -> Skeleton {
    let mut s = Skeleton::new_with_root();
    let mut t1 = Bone::new("tail1", Some("root".into()));
    t1.length = 64.0;
    let mut t2 = Bone::new("tail2", Some("tail1".into()));
    t2.length = 64.0;
    t2.x = 64.0;
    t2.rotation = tail2_rot;
    s.bones.push(t1);
    s.bones.push(t2);
    s.slots.push(Slot { attachment: Some("tail".into()), ..Slot::new("tail", "tail1") });
    #[rustfmt::skip]
    let vertices = vec![
        1.0, 1.0, 0.0, -16.0, 1.0,
        2.0, 1.0, 64.0, -16.0, 0.5, 2.0, 0.0, -16.0, 0.5,
        1.0, 2.0, 64.0, -16.0, 1.0,
        1.0, 2.0, 64.0, 16.0, 1.0,
        2.0, 1.0, 64.0, 16.0, 0.5, 2.0, 0.0, 16.0, 0.5,
        1.0, 1.0, 0.0, 16.0, 1.0,
    ];
    let mesh = MeshAttachment {
        uvs: vec![0.0, 1.0, 0.5, 1.0, 1.0, 1.0, 1.0, 0.0, 0.5, 0.0, 0.0, 0.0],
        triangles: vec![0, 1, 5, 1, 4, 5, 1, 2, 4, 2, 3, 4],
        weighted: true,
        vertices,
        hull: 6,
        width: 128.0,
        height: 32.0,
        ..Default::default()
    };
    s.skins[0].attachments.entry("tail").insert("tail", Attachment::Mesh(mesh));
    s
}

#[test]
fn f_d07_weighted_mesh_vertices() {
    // 셋업(tail2 회전 0): tail2 월드 = (64, 0) 회전 0 → 정점 1 = 0.5·(64,−16) + 0.5·(64+0, −16) = (64, −16)
    let out = PoseEngine::new().compute(&weighted_skel(0.0), &PoseRequest::default());
    let p = &out.draw[0].positions;
    assert!(close(p[2], 64.0) && close(p[3], -16.0));
    // tail2 회전 90°: (x, y) → (64 − y, x)
    // 정점 1 = 0.5·(64, −16) + 0.5·(64 + 16, 0) = (72, −8)
    // 정점 2 = tail2 (64, −16) → (64 + 16, 64) = (80, 64)
    // 정점 3 = tail2 (64, 16) → (64 − 16, 64) = (48, 64)
    // 정점 4 = 0.5·(64, 16) + 0.5·(64 − 16, 0) = (56, 8)
    let out = PoseEngine::new().compute(&weighted_skel(90.0), &PoseRequest::default());
    let p = &out.draw[0].positions;
    let want = [0.0, -16.0, 72.0, -8.0, 80.0, 64.0, 48.0, 64.0, 56.0, 8.0, 0.0, 16.0];
    for (a, b) in p.iter().zip(want) {
        assert!(close(*a, b), "{p:?}");
    }
    // 가중치 메시 deform: 본별 항목 순서로 2개씩. 정점 1의 tail1 항목(번호 1)에 (0, 8)을 더하면 0.5·8 = 4 위로
    let mut s = weighted_skel(90.0);
    let mut a = Animation::new("d");
    a.deform.entry("default").entry("tail").insert("tail", vec![Key { time: 0.0, v: vec![0.0, 8.0], offset: 2, ..Default::default() }]);
    s.animations.push(a);
    let out = PoseEngine::new().compute(&s, &req_at("d", 0.0));
    assert!(close(out.draw[0].positions[3], -4.0), "{:?}", out.draw[0].positions);
}

fn anim_skel() -> Skeleton {
    let mut s = Skeleton::new_with_root();
    let mut b = Bone::new("b", Some("root".into()));
    b.x = 1.0;
    b.rotation = 10.0;
    b.scale_x = 2.0;
    s.bones.push(b);
    s.bones.push(Bone::new("c", Some("b".into())));
    s
}

#[test]
fn f_g02_bone_timelines() {
    let mut s = anim_skel();
    let mut a = Animation::new("a");
    let tl = a.bones.entry("b");
    tl.insert("rotate", vec![Key::new(0.0, vec![0.0]), Key::new(1.0, vec![30.0])]);
    tl.insert("translate", vec![Key::new(0.0, vec![0.0, 0.0]), Key::new(2.0, vec![10.0, 20.0])]);
    tl.insert("scale", vec![Key::new(0.0, vec![1.0, 1.0]), Key::new(1.0, vec![2.0, 3.0])]);
    tl.insert("sheary", vec![Key::new(0.0, vec![0.0]), Key::new(1.0, vec![-8.0])]);
    tl.insert("translatey", vec![]);
    let tc = a.bones.entry("c");
    tc.insert("inherit", vec![Key { time: 0.0, s: Some("normal".into()), ..Default::default() }, Key { time: 0.5, s: Some("onlyTranslation".into()), ..Default::default() }]);
    tc.insert("shearx", vec![Key::new(0.0, vec![4.0])]);
    s.animations.push(a);
    let out = PoseEngine::new().compute(&s, &req_at("a", 0.5));
    let l = out.locals[1];
    // 회전 10 + 30·0.5 = 25, 위치 (1 + 2.5, 0 + 5), 크기 2·1.5 = 3, 1·2 = 2, shearY −4
    assert!(close(l.rotation, 25.0) && close(l.x, 3.5) && close(l.y, 5.0), "{l:?}");
    assert!(close(l.scale_x, 3.0) && close(l.scale_y, 2.0) && close(l.shear_y, -4.0), "{l:?}");
    assert!(close(out.locals[2].shear_x, 4.0));
    // inherit 계단: 0.5부터 onlyTranslation → 자식 X축 각 = 자기 shearX 4°만 (부모 회전 25° 무시)
    assert!(close(out.bones[2].rotation_x(), 4.0), "{}", out.bones[2].rotation_x());
    let out = PoseEngine::new().compute(&s, &req_at("a", 0.49));
    assert!(!close(out.bones[2].rotation_x(), 4.0));
    // 반복: 길이 2, 시점 2.5 → 0.5와 같음
    let looped = PoseEngine::new().compute(&s, &PoseRequest { looping: true, ..req_at("a", 2.5) });
    assert!(close(looped.locals[1].rotation, 25.0));
}

fn slot_skel() -> Skeleton {
    let mut s = Skeleton::new_with_root();
    for n in ["A", "B", "C"] {
        s.slots.push(Slot { attachment: Some("img".into()), dark: Some(DarkColor(Color::BLACK)), ..Slot::new(n, "root") });
        s.skins[0].attachments.entry(n).insert("img", Attachment::Region(RegionAttachment::default()));
        s.skins[0].attachments.entry(n).insert("img2", Attachment::Region(RegionAttachment::default()));
    }
    s
}

#[test]
fn f_g03_slot_color_and_attachment() {
    let mut s = slot_skel();
    let mut a = Animation::new("a");
    let tl = a.slots.entry("A");
    tl.insert("rgba", vec![Key::new(0.0, vec![1.0, 1.0, 1.0, 1.0]), Key::new(1.0, vec![0.0, 0.5, 1.0, 0.0])]);
    tl.insert("attachment", vec![Key { time: 0.0, s: Some("img2".into()), ..Default::default() }, Key { time: 1.0, s: None, ..Default::default() }]);
    let tb = a.slots.entry("B");
    tb.insert("rgb2", vec![Key::new(0.0, vec![0.2, 0.4, 0.6, 0.1, 0.2, 0.3])]);
    tb.insert("alpha", vec![Key::new(0.0, vec![0.0]), Key::new(2.0, vec![1.0])]);
    s.animations.push(a);
    let out = PoseEngine::new().compute(&s, &req_at("a", 0.5));
    // A: (1,1,1,1)→(0,0.5,1,0) 중간 = (0.5, 0.75, 1, 0.5), 첨부물 img2
    let d = &out.draw[0];
    assert_eq!((d.slot.as_str(), d.attachment.as_str()), ("A", "img2"));
    for (x, y) in d.color.iter().zip([0.5, 0.75, 1.0, 0.5]) {
        assert!(close(*x, y), "{:?}", d.color);
    }
    // B: rgb (0.2,0.4,0.6), 알파 0.25, 어두운 색 (0.1,0.2,0.3)
    let b = &out.draw[1];
    for (x, y) in b.color.iter().zip([0.2, 0.4, 0.6, 0.25]) {
        assert!(close(*x, y), "{:?}", b.color);
    }
    let dk = b.dark.unwrap();
    assert!(close(dk[0], 0.1) && close(dk[1], 0.2) && close(dk[2], 0.3));
    // 1초부터 A 첨부물 비움 → 그리지 않음
    let out = PoseEngine::new().compute(&s, &req_at("a", 1.0));
    assert!(out.draw.iter().all(|d| d.slot != "A"));
}

#[test]
fn f_g04_draw_order_keys() {
    let mut s = slot_skel();
    let mut a = Animation::new("a");
    a.draw_order.push(DrawOrderKey { time: 0.0, offsets: vec![] });
    a.draw_order.push(DrawOrderKey { time: 1.0, offsets: vec![DrawOrderOffset { slot: "A".into(), offset: 2 }] });
    s.animations.push(a);
    let names = |t: f32| PoseEngine::new().compute(&s, &req_at("a", t)).draw.iter().map(|d| d.slot.clone()).collect::<Vec<_>>();
    assert_eq!(names(0.5), vec!["A", "B", "C"]);
    // A를 2칸 뒤로: 자리 2에 A, 나머지는 B, C 순서로 빈 자리 → B, C, A
    assert_eq!(names(1.0), vec!["B", "C", "A"]);
}

#[test]
fn f_g05_deform_linear() {
    let mut s = Skeleton::new_with_root();
    s.slots.push(Slot { attachment: Some("m".into()), ..Slot::new("s", "root") });
    let Some(Attachment::Mesh(mesh)) = Attachment::new_of_type("mesh") else { panic!() };
    s.skins[0].attachments.entry("s").insert("m", Attachment::Mesh(mesh));
    let mut a = Animation::new("a");
    a.deform.entry("default").entry("s").insert(
        "m",
        vec![Key { time: 0.0, ..Default::default() }, Key { time: 1.0, v: vec![10.0, 20.0], offset: 2, ..Default::default() }],
    );
    s.animations.push(a);
    // 정점 1 셋업 (16, −16), 1초 차이 (10, 20) → 0.5초 = (16 + 5, −16 + 10) = (21, −6)
    let out = PoseEngine::new().compute(&s, &req_at("a", 0.5));
    let p = &out.draw[0].positions;
    assert!(close(p[2], 21.0) && close(p[3], -6.0), "{p:?}");
    assert!(close(p[0], -16.0) && close(p[1], -16.0));
}

#[test]
fn f_g06_constraint_timelines() {
    let mut s = anim_skel();
    s.bones.push(Bone::new("t", Some("root".into())));
    let ik = IkConstraint { name: "ik".into(), skin_required: false, bones: vec!["c".into()], target: "t".into(), mix: 1.0, softness: 0.0, bend_positive: true, compress: false, stretch: false, uniform: false, volume: false };
    s.constraints.push(Constraint::Ik(ik));
    let phys = |n: &str| PhysicsConstraint { name: n.into(), skin_required: false, bone: "c".into(), x: 0.0, y: 0.0, rotate: 0.0, scale_x: 0.0, shear_x: 0.0, limit: 5000.0, fps: 60.0, inertia: 1.0, strength: 100.0, damping: 1.0, mass: 1.0, wind: 0.0, gravity: 0.0, mix: 1.0, volume: false };
    s.constraints.push(Constraint::Physics(phys("p1")));
    s.constraints.push(Constraint::Physics(phys("p2")));
    let mut a = Animation::new("a");
    a.constraints.entry("ik").insert("ik", vec![Key::new(0.0, vec![1.0, 0.0, 1.0, 0.0, 0.0]), Key::new(1.0, vec![0.0, 4.0, -1.0, 1.0, 1.0])]);
    a.constraints.entry("").insert("gravity", vec![Key::new(0.0, vec![0.0]), Key::new(1.0, vec![10.0])]);
    a.constraints.entry("p2").insert("wind", vec![Key::new(0.0, vec![3.0])]);
    s.animations.push(a);
    let mut pose = SkeletonPose::setup(&s);
    apply_animation(&s, &s.animations[0], 0.5, &mut pose, ApplyOptions::default());
    // ik: mix 0.5, softness 2, 계단 값(bend, compress, stretch)은 앞 키 값 1, 0, 0
    assert_eq!(pose.constraint_values[0], vec![0.5, 2.0, 1.0, 0.0, 0.0]);
    // 이름 ""의 gravity는 두 물리 제약 모두 5. wind는 p2만 3
    assert!(close(pose.constraint_values[1][5], 5.0) && close(pose.constraint_values[2][5], 5.0));
    assert!(close(pose.constraint_values[1][4], 0.0) && close(pose.constraint_values[2][4], 3.0));
    // mix 0.5로 섞기: 셋업 1에서 0.5 쪽으로 반 → 0.75
    let mut pose = SkeletonPose::setup(&s);
    apply_animation(&s, &s.animations[0], 0.5, &mut pose, ApplyOptions { alpha: 0.5, additive: false });
    assert!(close(pose.constraint_values[0][0], 0.75));
    let w = compute_world(&s, &pose.locals, &pose.inherits);
    assert_eq!(w.len(), 4);
}

#[test]
fn full_sample_computes() {
    // 형식 담당의 샘플을 읽어 모든 기능이 한 번에 돌아가는지(패닉 없음, 값이 유한) 확인한다.
    let text = include_str!("../../../../samples/kgbone/full-4.3.kgbone");
    let v = crate::json::parse(text).unwrap();
    let p: Project = crate::json::FromJson::from_json(&v).unwrap();
    let mut e = PoseEngine::new();
    for anim in std::iter::once(None).chain(p.skeleton.animations.iter().map(|a| Some(a.name.clone()))) {
        for i in 0..30 {
            let req = PoseRequest { animation: anim.clone(), time: i as f32 / 30.0, last_time: Some((i as f32 - 1.0) / 30.0), physics: super::PhysicsStep::Advance(1.0 / 30.0), looping: true, skin: None };
            let out = e.compute(&p.skeleton, &req);
            assert!(out.bones.iter().all(|b| b.a.is_finite() && b.x.is_finite() && b.y.is_finite()));
            assert!(out.draw.iter().all(|d| d.positions.iter().all(|x| x.is_finite())));
        }
    }
}
