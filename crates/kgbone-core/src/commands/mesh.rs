//! 메시 정점·삼각형·가중치 명령.
//!
//! 위치 변환은 모두 셋업 포즈(제약 포함, `pose::setup_world`) 기준이다. 애니메이트 모드의 `mesh.setVertices`만
//! 현재 애니메이션·시점의 포즈를 쓴다. 정점 수나 표기(가중치 유무)가 바뀌는 편집(generate, bind, unbind,
//! 정점 수가 바뀐 setVertices)은 그 첨부물의 deform 키를 지운다(길이가 맞지 않게 되므로).

use super::{cmd, Args, CommandContext, FnCommand};
use crate::json::JValue as Value;
use crate::mesh::{self, BoneInfo, PaintMode};
use crate::model::{Attachment, EditorMode, Key, MeshAttachment, Project, Skeleton, Skin};
use crate::pose::{self, PoseRequest, WorldBone};
use crate::{CoreError, Result};

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("mesh.setVertices", "메시 정점 편집", mesh_set_vertices),
    cmd("mesh.setTriangles", "메시 삼각형 편집", mesh_set_triangles),
    cmd("mesh.generate", "메시 자동 생성", mesh_generate),
    cmd("mesh.bind", "가중치 바인딩", mesh_bind),
    cmd("mesh.unbind", "가중치 해제", mesh_unbind),
    cmd("weights.auto", "자동 가중치", weights_auto),
    cmd("weights.paint", "가중치 페인팅", weights_paint),
];

/// 명령 대상(스킨·슬롯·첨부물 이름).
struct Target {
    skin: String,
    slot: String,
    name: String,
}

impl Target {
    fn from(args: &Args) -> Result<Target> {
        Ok(Target { skin: args.opt_str("skin").unwrap_or(Skin::DEFAULT).to_string(), slot: args.str("slot")?.to_string(), name: args.str("name")?.to_string() })
    }

    fn mesh<'a>(&self, skel: &'a Skeleton) -> Result<&'a MeshAttachment> {
        let skin = skel.skin(&self.skin).ok_or_else(|| CoreError::NotFound(format!("스킨 '{}'", self.skin)))?;
        match skin.get(&self.slot, &self.name) {
            Some(Attachment::Mesh(m)) => Ok(m),
            Some(_) => Err(CoreError::Rejected(format!("'{}'은(는) 메시가 아닙니다", self.name))),
            None => Err(CoreError::NotFound(format!("첨부물 '{}/{}'", self.slot, self.name))),
        }
    }

    fn mesh_mut<'a>(&self, skel: &'a mut Skeleton) -> Result<&'a mut MeshAttachment> {
        self.mesh(skel)?;
        match skel.skin_mut(&self.skin).and_then(|s| s.get_mut(&self.slot, &self.name)) {
            Some(Attachment::Mesh(m)) => Ok(m),
            _ => Err(CoreError::NotFound(format!("첨부물 '{}/{}'", self.slot, self.name))),
        }
    }

    fn slot_bone(&self, skel: &Skeleton, world: &[WorldBone]) -> Result<WorldBone> {
        let slot = skel.slot(&self.slot).ok_or_else(|| CoreError::NotFound(format!("슬롯 '{}'", self.slot)))?;
        let bi = skel.bone_index(&slot.bone).ok_or_else(|| CoreError::NotFound(format!("본 '{}'", slot.bone)))?;
        Ok(world[bi])
    }

    /// 이 첨부물의 deform 키를 모든 애니메이션에서 지운다.
    fn drop_deform_keys(&self, skel: &mut Skeleton) {
        for a in &mut skel.animations {
            if let Some(slots) = a.deform.get_mut(&self.skin) {
                if let Some(atts) = slots.get_mut(&self.slot) {
                    atts.remove(&self.name);
                }
            }
        }
    }
}

/// 셋업 포즈에서 메시 정점의 월드 좌표.
fn setup_world_vertices(skel: &Skeleton, t: &Target, world: &[WorldBone]) -> Result<Vec<f32>> {
    let m = t.mesh(skel)?;
    let sb = t.slot_bone(skel, world)?;
    Ok(pose::vertex_world_positions(m.weighted, &m.vertices, &sb, world, None))
}

/// 메시 정점의 슬롯 본 기준 로컬 좌표(가중치면 셋업 포즈 위치를 되돌림).
fn local_vertices(skel: &Skeleton, t: &Target, world: &[WorldBone]) -> Result<Vec<f32>> {
    let m = t.mesh(skel)?;
    if !m.weighted {
        return Ok(m.vertices.clone());
    }
    Ok(mesh::unbind_vertices(&m.vertices, world, &t.slot_bone(skel, world)?))
}

fn bone_list(skel: &Skeleton, names: &[String]) -> Result<Vec<usize>> {
    names.iter().map(|n| skel.bone_index(n).ok_or_else(|| CoreError::NotFound(format!("본 '{n}'")))).collect()
}

fn bone_infos(skel: &Skeleton, bones: &[usize], world: &[WorldBone]) -> Vec<BoneInfo> {
    bones.iter().map(|&i| BoneInfo { index: i, world: world[i], length: skel.bones[i].length }).collect()
}

/// 외곽선(앞 `hull`개 정점)을 귀 자르기로 삼각분할.
fn hull_triangles(local: &[f32], hull: u32) -> Vec<u32> {
    let n = local.len() / 2;
    let h = if hull >= 3 && (hull as usize) <= n { hull as usize } else { n };
    mesh::triangulate(&local[..h * 2])
}

/// 정점 좌표로 uv를 만든다(첨부물 폭·높이 기준, 중심 원점).
fn uvs_from_positions(local: &[f32], w: f32, h: f32) -> Vec<f32> {
    let (w, h) = (if w.abs() > 1e-6 { w } else { 1.0 }, if h.abs() > 1e-6 { h } else { 1.0 });
    local.chunks_exact(2).flat_map(|p| [p[0] / w + 0.5, 0.5 - p[1] / h]).collect()
}

/// 키 목록에 시점 `time`의 키를 넣거나(같은 시점이 있으면) 고친다.
fn upsert_key(keys: &mut Vec<Key>, time: f32, v: Vec<f32>) {
    if let Some(k) = keys.iter_mut().find(|k| (k.time - time).abs() < 1e-4) {
        k.v = v;
        k.offset = 0;
        return;
    }
    let pos = keys.iter().position(|k| k.time > time).unwrap_or(keys.len());
    keys.insert(pos, Key { time, v, ..Default::default() });
}

fn animate_target(p: &Project) -> Option<(String, f32)> {
    match (&p.editor.mode, &p.editor.active_animation) {
        (EditorMode::Animate, Some(a)) if p.skeleton.animation(a).is_some() => Some((a.clone(), p.editor.time)),
        _ => None,
    }
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string", "vertices": "number[] (가중치 없는 로컬 x,y)", "uvs?": "number[]", "hull?": "number"}
fn mesh_set_vertices(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let t = Target::from(args)?;
    let verts = args.f32_list("vertices").ok_or_else(|| CoreError::InvalidArgs("'vertices' 숫자 배열이 필요합니다".into()))?;
    if verts.len() % 2 != 0 {
        return Err(CoreError::InvalidArgs("vertices는 x, y 쌍이어야 합니다".into()));
    }
    let skel = &ctx.project.skeleton;
    let m = t.mesh(skel)?.clone();
    let count = if m.weighted { mesh::read_weights(&m.vertices).len() } else { m.vertices.len() / 2 };

    if let Some((anim, time)) = animate_target(ctx.project) {
        if verts.len() / 2 != count {
            return Err(CoreError::Rejected("애니메이트 모드에서는 정점 수를 바꿀 수 없습니다".into()));
        }
        let deform = if !m.weighted {
            verts.iter().zip(&m.vertices).map(|(a, b)| a - b).collect::<Vec<f32>>()
        } else {
            // 현재 포즈에서: 목표 월드 − 현재(변형 포함) 월드 = Δ, 본별 오프셋에 Mᵦ⁻¹·Δ를 더한다.
            let req = PoseRequest { animation: Some(anim.clone()), time, skin: ctx.project.editor.active_skin.clone(), ..Default::default() };
            let mut st = Default::default();
            let pose_now = pose::compute_pose(skel, &req, &mut st);
            let sb = t.slot_bone(skel, &pose_now.world)?;
            let old = pose_now.deform.get(&(t.slot.clone(), t.name.clone())).cloned().unwrap_or_default();
            let cur = pose::vertex_world_positions(true, &m.vertices, &sb, &pose_now.world, Some(&old));
            let mut out = vec![0.0f32; pose::deform_len(true, &m.vertices)];
            for (k, o) in old.iter().enumerate().take(out.len()) {
                out[k] = *o;
            }
            let weights = mesh::read_weights(&m.vertices);
            let mut entry = 0usize;
            for (vi, w) in weights.iter().enumerate() {
                let (tx, ty) = sb.local_to_world(verts[2 * vi], verts[2 * vi + 1]);
                let (dx, dy) = (tx - cur[2 * vi], ty - cur[2 * vi + 1]);
                for (b, _) in w {
                    let bw = pose_now.world.get(*b).copied().unwrap_or_default();
                    let lin = WorldBone { x: 0.0, y: 0.0, ..bw };
                    let (lx, ly) = lin.world_to_local(dx, dy);
                    out[2 * entry] += lx;
                    out[2 * entry + 1] += ly;
                    entry += 1;
                }
            }
            out
        };
        let a = ctx.project.skeleton.animation_mut(&anim).ok_or_else(|| CoreError::NotFound(format!("애니메이션 '{anim}'")))?;
        let keys = a.deform.entry(&t.skin).entry(&t.slot).entry(&t.name);
        upsert_key(keys, time, deform);
        return Ok(Value::Null);
    }

    // 셋업 모드
    let world = pose::setup_world(skel);
    if m.weighted {
        if verts.len() / 2 != count {
            return Err(CoreError::Rejected("가중치 메시는 정점 수를 바꿀 수 없습니다. 먼저 가중치를 해제하세요".into()));
        }
        let sb = t.slot_bone(skel, &world)?;
        let cur = pose::vertex_world_positions(true, &m.vertices, &sb, &world, None);
        let mut new_v = m.vertices.clone();
        let mut i = 0usize;
        for vi in 0..count {
            let n = new_v[i] as usize;
            i += 1;
            let (tx, ty) = sb.local_to_world(verts[2 * vi], verts[2 * vi + 1]);
            let (dx, dy) = (tx - cur[2 * vi], ty - cur[2 * vi + 1]);
            for _ in 0..n {
                let bw = world.get(new_v[i] as usize).copied().unwrap_or_default();
                let lin = WorldBone { x: 0.0, y: 0.0, ..bw };
                let (lx, ly) = lin.world_to_local(dx, dy);
                new_v[i + 1] += lx;
                new_v[i + 2] += ly;
                i += 4;
            }
        }
        t.mesh_mut(&mut ctx.project.skeleton)?.vertices = new_v;
        return Ok(Value::Null);
    }
    let changed_count = verts.len() / 2 != count;
    let uvs = args.f32_list("uvs");
    let hull = args.opt_usize("hull").map(|h| h as u32);
    if let Some(u) = &uvs {
        if u.len() != verts.len() {
            return Err(CoreError::InvalidArgs("uvs 길이가 vertices와 같아야 합니다".into()));
        }
    }
    let mm = t.mesh_mut(&mut ctx.project.skeleton)?;
    mm.vertices = verts.clone();
    if let Some(h) = hull {
        mm.hull = h.min((verts.len() / 2) as u32);
    }
    match uvs {
        Some(u) => mm.uvs = u,
        None if changed_count => mm.uvs = uvs_from_positions(&verts, mm.width, mm.height),
        None => {}
    }
    if changed_count {
        if mm.hull as usize > verts.len() / 2 || hull.is_none() {
            mm.hull = (verts.len() / 2) as u32;
        }
        mm.triangles = hull_triangles(&verts, mm.hull);
        mm.edges.clear();
        t.drop_deform_keys(&mut ctx.project.skeleton);
    }
    Ok(Value::Null)
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string", "triangles?": "number[] (없으면 외곽선으로 자동 삼각분할)"}
fn mesh_set_triangles(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let t = Target::from(args)?;
    let skel = &ctx.project.skeleton;
    let m = t.mesh(skel)?;
    let n = if m.weighted { mesh::read_weights(&m.vertices).len() } else { m.vertices.len() / 2 };
    let tris = match args.u32_list("triangles") {
        Some(tr) => {
            if tr.len() % 3 != 0 || tr.iter().any(|&i| i as usize >= n) {
                return Err(CoreError::InvalidArgs("triangles는 정점 번호 3개씩이어야 합니다".into()));
            }
            tr
        }
        None => {
            let world = pose::setup_world(skel);
            hull_triangles(&local_vertices(skel, &t, &world)?, m.hull)
        }
    };
    let count = tris.len() / 3;
    t.mesh_mut(&mut ctx.project.skeleton)?.triangles = tris;
    Ok(Value::obj().with("triangleCount", Value::num(count as f64)))
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string", "detail?": "number(0~1)"}
fn mesh_generate(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let t = Target::from(args)?;
    let m = t.mesh(&ctx.project.skeleton)?;
    let path = m.path.clone().unwrap_or_else(|| t.name.clone());
    let info = ctx.images.get(&path).or_else(|| ctx.images.get(&t.name));
    let Some((alpha, w, h)) = info.and_then(|i| i.alpha.as_ref().map(|a| (a, i.width, i.height))) else {
        return Err(CoreError::Rejected(format!("이미지 알파 정보가 없습니다: '{path}' 이미지를 먼저 불러오세요")));
    };
    let g = mesh::generate_from_alpha(alpha, args.opt_f32("detail").unwrap_or(0.5));
    if g.triangles.is_empty() {
        return Err(CoreError::Rejected("불투명한 부분이 없어 메시를 만들 수 없습니다".into()));
    }
    let (vc, tc) = (g.vertices.len() / 2, g.triangles.len() / 3);
    let mm = t.mesh_mut(&mut ctx.project.skeleton)?;
    mm.vertices = g.vertices;
    mm.uvs = g.uvs;
    mm.triangles = g.triangles;
    mm.hull = g.hull;
    mm.weighted = false;
    mm.edges.clear();
    mm.width = w as f32;
    mm.height = h as f32;
    t.drop_deform_keys(&mut ctx.project.skeleton);
    Ok(Value::obj().with("vertexCount", Value::num(vc as f64)).with("triangleCount", Value::num(tc as f64)))
}

/// 본 목록으로 자동 가중치를 계산해 바인딩한다(셋업 월드 위치 유지).
fn bind_with(ctx: &mut CommandContext, t: &Target, bones: &[usize]) -> Result<()> {
    if bones.is_empty() {
        return Err(CoreError::InvalidArgs("바인딩할 본을 하나 이상 고르세요".into()));
    }
    let skel = &ctx.project.skeleton;
    let world = pose::setup_world(skel);
    let wv = setup_world_vertices(skel, t, &world)?;
    let was_weighted = t.mesh(skel)?.weighted;
    let weights = mesh::auto_weights(&wv, &bone_infos(skel, bones, &world));
    let bound = mesh::bind_vertices(&wv, &weights, &world);
    let mm = t.mesh_mut(&mut ctx.project.skeleton)?;
    mm.vertices = bound;
    mm.weighted = true;
    // 가중치 메시의 deform 길이는 본별 항목 수에 따라 달라진다.
    let _ = was_weighted;
    t.drop_deform_keys(&mut ctx.project.skeleton);
    Ok(())
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string", "bones": "string[]"}
fn mesh_bind(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let t = Target::from(args)?;
    let bones = bone_list(&ctx.project.skeleton, &args.str_list("bones"))?;
    bind_with(ctx, &t, &bones)?;
    Ok(Value::Null)
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string"}
fn mesh_unbind(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let t = Target::from(args)?;
    let skel = &ctx.project.skeleton;
    if !t.mesh(skel)?.weighted {
        return Ok(Value::Null);
    }
    let world = pose::setup_world(skel);
    let local = local_vertices(skel, &t, &world)?;
    let mm = t.mesh_mut(&mut ctx.project.skeleton)?;
    mm.vertices = local;
    mm.weighted = false;
    t.drop_deform_keys(&mut ctx.project.skeleton);
    Ok(Value::Null)
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string", "bones?": "string[]"}
fn weights_auto(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let t = Target::from(args)?;
    let skel = &ctx.project.skeleton;
    let names = args.str_list("bones");
    let bones = if !names.is_empty() {
        bone_list(skel, &names)?
    } else {
        let m = t.mesh(skel)?;
        if !m.weighted {
            return Err(CoreError::Rejected("가중치를 줄 본을 고르세요(바인딩되지 않은 메시)".into()));
        }
        let mut b: Vec<usize> = mesh::read_weights(&m.vertices).into_iter().flatten().map(|(b, _)| b).collect();
        b.sort_unstable();
        b.dedup();
        b
    };
    bind_with(ctx, &t, &bones)?;
    Ok(Value::Null)
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string", "bone": "string", "x": "number(월드)", "y": "number(월드)", "radius": "number", "strength": "number(0~1)", "mode?": "add|remove|replace"}
fn weights_paint(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let t = Target::from(args)?;
    let skel = &ctx.project.skeleton;
    let bone_name = args.str("bone")?;
    let bone = skel.bone_index(bone_name).ok_or_else(|| CoreError::NotFound(format!("본 '{bone_name}'")))?;
    let mode = match args.opt_str("mode").unwrap_or("add") {
        "add" => PaintMode::Add,
        "remove" => PaintMode::Remove,
        "replace" => PaintMode::Replace,
        other => return Err(CoreError::InvalidArgs(format!("모르는 붓 모드: {other}"))),
    };
    let m = t.mesh(skel)?;
    if !m.weighted {
        return Err(CoreError::Rejected("가중치가 없는 메시입니다. 먼저 바인딩하세요".into()));
    }
    let world = pose::setup_world(skel);
    let wv = setup_world_vertices(skel, &t, &world)?;
    let mut weights = mesh::read_weights(&m.vertices);
    let before = weights.clone();
    let strength = args.f32("strength")?.clamp(0.0, 1.0);
    mesh::paint_weights(&mut weights, &wv, bone, args.f32("x")?, args.f32("y")?, args.f32("radius")?, strength, mode);
    let changed = weights.iter().zip(&before).filter(|(a, b)| a != b).count();
    let bound = mesh::bind_vertices(&wv, &weights, &world);
    t.mesh_mut(&mut ctx.project.skeleton)?.vertices = bound;
    t.drop_deform_keys(&mut ctx.project.skeleton);
    Ok(Value::obj().with("changed", Value::num(changed as f64)))
}

#[cfg(test)]
mod tests {
    use crate::json::{self, JValue};
    use crate::model::{Attachment, Bone, EditorMode, Project, Slot};
    use crate::pose::{PoseEngine, PoseOutput, PoseRequest};

    /// 명령을 프로젝트에 바로 실행하는 작은 시험용 문서(.kgbone 읽기는 형식 담당 구현 전이라 JSON 트리로 만든다).
    struct Document {
        project: Project,
        images: super::super::ImageStore,
    }

    impl Document {
        fn execute(&mut self, id: &str, a: &JValue) -> crate::Result<JValue> {
            let c = super::super::find(id).expect("등록된 명령");
            let mut ctx = super::super::CommandContext { project: &mut self.project, images: &self.images };
            c.run(&mut ctx, a)
        }
        fn pose(&mut self, req: &PoseRequest) -> PoseOutput {
            PoseEngine::new().compute(&self.project.skeleton, req)
        }
        fn project(&self) -> &Project {
            &self.project
        }
        fn set_image(&mut self, path: &str, w: u32, h: u32, alpha: Option<Vec<u8>>) {
            self.images.set(path, w, h, alpha);
        }
    }

    fn doc() -> Document {
        let text = r#"{
	"formatVersion": 1,
	"skeleton": {
		"bones": [
			{ "name": "root" },
			{ "name": "a", "parent": "root", "length": 50, "x": 10, "rotation": 30 },
			{ "name": "b", "parent": "a", "length": 50, "x": 50, "rotation": -45 }
		],
		"slots": [ { "name": "s", "bone": "a", "attachment": "m" } ],
		"skins": [ { "name": "default", "attachments": { "s": { "m": {
			"type": "mesh", "width": 100, "height": 20,
			"uvs": [0, 1, 0.5, 1, 1, 1, 1, 0, 0.5, 0, 0, 0],
			"triangles": [0, 1, 5, 1, 4, 5, 1, 2, 4, 2, 3, 4],
			"vertices": [0, -10, 50, -10, 100, -10, 100, 10, 50, 10, 0, 10],
			"hull": 6
		} } } } ],
		"animations": [ { "name": "anim" } ]
	}
}"#;
        let project: Project = crate::json::FromJson::from_json(&json::parse(text).unwrap()).unwrap();
        Document { project, images: Default::default() }
    }

    fn args(s: &str) -> JValue {
        json::parse(s).unwrap()
    }

    fn world_verts(d: &mut Document) -> Vec<f32> {
        d.pose(&crate::pose::PoseRequest::default()).draw[0].positions.clone()
    }

    fn mesh(d: &Document) -> crate::model::MeshAttachment {
        match d.project().skeleton.skins[0].get("s", "m") {
            Some(Attachment::Mesh(m)) => m.clone(),
            _ => panic!("메시 없음"),
        }
    }

    #[test]
    fn f_d04_bind_keeps_setup_world() {
        let mut d = doc();
        let before = world_verts(&mut d);
        d.execute("mesh.bind", &args(r#"{"slot": "s", "name": "m", "bones": ["a", "b"]}"#)).unwrap();
        assert!(mesh(&d).weighted);
        let after = world_verts(&mut d);
        for (x, y) in before.iter().zip(&after) {
            assert!((x - y).abs() < 1e-3, "{before:?} {after:?}");
        }
        // 해제도 위치 유지
        d.execute("mesh.unbind", &args(r#"{"slot": "s", "name": "m"}"#)).unwrap();
        let un = world_verts(&mut d);
        for (x, y) in before.iter().zip(&un) {
            assert!((x - y).abs() < 1e-3);
        }
    }

    #[test]
    fn f_d05_paint_command_sums_to_one() {
        let mut d = doc();
        d.execute("mesh.bind", &args(r#"{"slot": "s", "name": "m", "bones": ["a", "b"]}"#)).unwrap();
        let before = world_verts(&mut d);
        let w = &before;
        let r = d.execute("weights.paint", &args(&format!(r#"{{"slot": "s", "name": "m", "bone": "b", "x": {}, "y": {}, "radius": 60, "strength": 0.8, "mode": "add"}}"#, w[2], w[3]))).unwrap();
        assert!(r.get("changed").and_then(JValue::as_f64).unwrap() > 0.0);
        for vw in crate::mesh::read_weights(&mesh(&d).vertices) {
            let s: f32 = vw.iter().map(|x| x.1).sum();
            assert!((s - 1.0).abs() < 1e-4);
        }
        // 붓질 후에도 셋업 위치는 그대로
        let after = world_verts(&mut d);
        for (x, y) in before.iter().zip(&after) {
            assert!((x - y).abs() < 1e-3);
        }
    }

    #[test]
    fn f_d06_auto_weights_command() {
        let mut d = doc();
        assert!(d.execute("weights.auto", &args(r#"{"slot": "s", "name": "m"}"#)).is_err());
        d.execute("weights.auto", &args(r#"{"slot": "s", "name": "m", "bones": ["a", "b"]}"#)).unwrap();
        let w = crate::mesh::read_weights(&mesh(&d).vertices);
        // 정점 0(본 a 원점 근처)은 a가, 정점 2(본 b 끝 근처)는 b가 크다
        let get = |v: &Vec<(usize, f32)>, b: usize| v.iter().find(|x| x.0 == b).map(|x| x.1).unwrap_or(0.0);
        assert!(get(&w[0], 1) > get(&w[0], 2));
        assert!(get(&w[2], 2) > get(&w[2], 1));
    }

    #[test]
    fn set_vertices_setup_and_animate() {
        let mut d = doc();
        // 셋업: 비가중치 교체
        d.execute("mesh.setVertices", &args(r#"{"slot": "s", "name": "m", "vertices": [0, -12, 50, -10, 100, -10, 100, 10, 50, 10, 0, 10]}"#)).unwrap();
        assert_eq!(mesh(&d).vertices[1], -12.0);
        // 가중치 메시: 주어진 로컬 위치로 옮겨지고 가중치는 유지
        d.execute("mesh.bind", &args(r#"{"slot": "s", "name": "m", "bones": ["a", "b"]}"#)).unwrap();
        let w0 = crate::mesh::read_weights(&mesh(&d).vertices);
        d.execute("mesh.setVertices", &args(r#"{"slot": "s", "name": "m", "vertices": [0, -20, 50, -10, 100, -10, 100, 10, 50, 10, 0, 10]}"#)).unwrap();
        let w1 = crate::mesh::read_weights(&mesh(&d).vertices);
        assert_eq!(w0, w1);
        let p = world_verts(&mut d);
        let sb = crate::pose::setup_world(&d.project().skeleton)[1];
        let (x, y) = sb.local_to_world(0.0, -20.0);
        assert!((p[0] - x).abs() < 1e-3 && (p[1] - y).abs() < 1e-3);
        // 애니메이트 모드: 셋업은 그대로, deform 키 생성
        d.execute("mesh.unbind", &args(r#"{"slot": "s", "name": "m"}"#)).unwrap();
        let setup_v = mesh(&d).vertices.clone();
        let mut proj = d.project().clone();
        proj.editor.mode = EditorMode::Animate;
        proj.editor.active_animation = Some("anim".into());
        proj.editor.time = 0.5;
        let mut images = super::super::ImageStore::default();
        images.set("x", 1, 1, None);
        let mut ctx = super::super::CommandContext { project: &mut proj, images: &images };
        let mut v = setup_v.clone();
        v[0] += 3.0;
        let a = JValue::obj().with("slot", "s".into()).with("name", "m".into()).with("vertices", JValue::Arr(v.iter().map(|x| JValue::num(*x as f64)).collect()));
        super::mesh_set_vertices(&mut ctx, &super::Args::new(&a)).unwrap();
        let keys = proj.skeleton.animation("anim").unwrap().deform.get("default").unwrap().get("s").unwrap().get("m").unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].time, 0.5);
        assert!((keys[0].v[0] - 3.0).abs() < 1e-5);
        assert_eq!(proj.skeleton.skins[0].get("s", "m").map(|a| a.vertices().unwrap().1.to_vec()), Some(setup_v));
        let _ = (Bone::new("x", None), Slot::new("x", "y"));
    }

    #[test]
    fn set_triangles_and_generate() {
        let mut d = doc();
        let r = d.execute("mesh.setTriangles", &args(r#"{"slot": "s", "name": "m"}"#)).unwrap();
        assert_eq!(r.get("triangleCount").and_then(JValue::as_f64), Some(4.0));
        assert!(d.execute("mesh.setTriangles", &args(r#"{"slot": "s", "name": "m", "triangles": [0, 1, 9]}"#)).is_err());
        // 알파 없음 → 거부
        let e = d.execute("mesh.generate", &args(r#"{"slot": "s", "name": "m"}"#)).unwrap_err();
        assert!(e.to_string().contains("이미지 알파 정보가 없습니다"));
        let (w, h) = (20u32, 10u32);
        let alpha: Vec<u8> = (0..w * h).map(|i| if (2..18).contains(&(i % w)) && (2..8).contains(&(i / w)) { 255 } else { 0 }).collect();
        d.set_image("m", w, h, Some(alpha));
        let r = d.execute("mesh.generate", &args(r#"{"slot": "s", "name": "m", "detail": 0.2}"#)).unwrap();
        assert!(r.get("triangleCount").and_then(JValue::as_f64).unwrap() >= 2.0);
        let m = mesh(&d);
        assert_eq!((m.width, m.height), (20.0, 10.0));
        assert!(!m.weighted);
    }
}
