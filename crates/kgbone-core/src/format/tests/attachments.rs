//! 첨부물 편집·왕복 테스트: D02(메시 정점·삼각형 편집), D09(바운딩 박스), D12(패스).
use crate::json::{parse, JValue};
use crate::model::{Attachment, Skin};
use crate::Document;

fn run(doc: &mut Document, id: &str, args: &str) -> JValue {
    doc.execute(id, &parse(args).unwrap()).unwrap_or_else(|e| panic!("{id} 실패: {e}"))
}

fn att<'a>(doc: &'a Document, slot: &str, name: &str) -> &'a Attachment {
    doc.project().skeleton.skin(Skin::DEFAULT).unwrap().get(slot, name).unwrap()
}

/// 저장 → 다시 열기 후 같은 문서인지.
fn reopen(doc: &mut Document) -> Document {
    let bytes = doc.save_kgbone().unwrap();
    Document::open_kgbone(&bytes).unwrap().0
}

fn setup() -> Document {
    let mut doc = Document::new();
    run(&mut doc, "slot.create", r#"{"bone": "root", "name": "s"}"#);
    doc
}

#[test]
fn f_d02_mesh_vertices_and_triangles_edit() {
    let mut doc = setup();
    run(&mut doc, "attachment.create", r#"{"slot": "s", "name": "m", "type": "mesh"}"#);
    // 오각형으로 정점 바꾸기(uv도 함께), 삼각형은 자동(외곽선 귀 자르기)
    run(
        &mut doc,
        "mesh.setVertices",
        r#"{"slot": "s", "name": "m", "vertices": [0, 0, 10, 0, 14, 8, 5, 14, -4, 8],
            "uvs": [0.3, 1, 1, 1, 1, 0.4, 0.5, 0, 0, 0.4], "hull": 5}"#,
    );
    run(&mut doc, "mesh.setTriangles", r#"{"slot": "s", "name": "m"}"#);
    let Attachment::Mesh(m) = att(&doc, "s", "m") else { panic!("메시가 아님") };
    assert_eq!(m.vertices.len(), 10);
    // 정점 5개 단순 다각형 → 삼각형 3개
    assert_eq!(m.triangles.len(), 9);
    assert!(m.triangles.iter().all(|&i| (i as usize) < m.vertex_count()), "삼각형 번호가 정점 수를 넘음: {:?}", m.triangles);
    // 직접 지정한 삼각형도 같은 규칙
    run(&mut doc, "mesh.setTriangles", r#"{"slot": "s", "name": "m", "triangles": [0, 1, 2, 0, 2, 3, 0, 3, 4]}"#);
    let before = att(&doc, "s", "m").clone();
    // 다시 열어도 같다
    let doc2 = reopen(&mut doc);
    assert_eq!(att(&doc2, "s", "m"), &before);
    // 되돌리기 한 번 = 직접 지정 전 삼각형
    doc.execute("edit.undo", &JValue::obj()).unwrap();
    let Attachment::Mesh(m) = att(&doc, "s", "m") else { panic!() };
    assert_ne!(m.triangles, vec![0, 1, 2, 0, 2, 3, 0, 3, 4]);
}

#[test]
fn f_d09_bounding_box_roundtrip() {
    let mut doc = setup();
    run(
        &mut doc,
        "attachment.create",
        r#"{"slot": "s", "name": "hit", "type": "boundingbox", "props": {"vertexCount": 3, "vertices": [0, 0, 20, 0, 10, 15], "color": "112233FF"}}"#,
    );
    let doc2 = reopen(&mut doc);
    let Attachment::BoundingBox(b) = att(&doc2, "s", "hit") else { panic!("바운딩 박스가 아님") };
    assert_eq!(b.vertex_count, 3);
    assert_eq!(b.vertices, vec![0.0, 0.0, 20.0, 0.0, 10.0, 15.0]);
    assert_eq!(b.color.to_hex_rgba(), "112233FF");
    // Spine JSON 호환 내보내기 → 가져오기에서도 같다
    let exported = doc2.export_spine_json(&Default::default()).unwrap();
    let (doc3, _) = Document::import_spine_json(&exported.bytes).unwrap();
    assert_eq!(att(&doc3, "s", "hit"), att(&doc2, "s", "hit"));
}

#[test]
fn f_d12_path_attachment_roundtrip() {
    let mut doc = setup();
    run(
        &mut doc,
        "attachment.create",
        r#"{"slot": "s", "name": "p", "type": "path", "props": {"closed": true, "constantSpeed": false, "vertexCount": 6,
            "vertices": [-10, 0, 0, 0, 10, 5, 40, 5, 50, 0, 60, 0], "lengths": [55.5, 120]}}"#,
    );
    let doc2 = reopen(&mut doc);
    let Attachment::Path(p) = att(&doc2, "s", "p") else { panic!("패스가 아님") };
    assert!(p.closed);
    assert!(!p.constant_speed);
    assert_eq!(p.lengths, vec![55.5, 120.0]);
    assert_eq!(p.vertex_count, 6);
    let exported = doc2.export_spine_json(&Default::default()).unwrap();
    let (doc3, _) = Document::import_spine_json(&exported.bytes).unwrap();
    assert_eq!(att(&doc3, "s", "p"), att(&doc2, "s", "p"));
}
