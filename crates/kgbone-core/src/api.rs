//! 화면(WebAssembly)과 앱 본체가 같은 모양으로 핵심 모듈을 부르는 JSON 요청 처리기.
//!
//! 요청: `{"method": 이름, ...인자}` → 응답: `{"ok": 결과}` 또는 `{"error": 한국어 메시지}`.
//! 메서드 목록은 `interface/core-api.md`와 `app/src/core/bridge.ts`에 같은 이름으로 적혀 있다.
use crate::document::Document;
use crate::format::spine_json::{SpineExportOptions, SpineVersion};
use crate::json::{self, FromJson, JValue, ToJson};
use crate::pose::{PhysicsStep, PoseRequest};
use crate::{CoreError, Result};

/// 문서 하나를 들고 요청을 처리한다.
#[derive(Default)]
pub struct Session {
    doc: Document,
    /// 이미지 경로 → PNG 원본 바이트(내보내기용).
    pngs: std::collections::HashMap<String, Vec<u8>>,
}

impl crate::export::ImageSource for Session {
    fn load(&self, path: &str) -> Option<crate::export::Image> {
        crate::export::decode_png(self.pngs.get(path)?).ok()
    }
}

fn bundle_json(b: crate::export::Bundle) -> JValue {
    let files = b
        .files
        .iter()
        .map(|(n, bytes)| JValue::obj().with("name", JValue::Str(n.clone())).with("base64", JValue::Str(crate::export::base64::encode(bytes))))
        .collect();
    JValue::obj().with("files", JValue::Arr(files)).with("warnings", b.warnings.to_json(true))
}

impl Session {
    pub fn new() -> Self {
        Session::default()
    }

    pub fn document(&self) -> &Document {
        &self.doc
    }

    /// 요청 텍스트 → 응답 텍스트(한 줄 JSON).
    pub fn handle_text(&mut self, request: &str) -> String {
        let resp = match json::parse(request) {
            Ok(req) => match self.handle(&req) {
                Ok(v) => JValue::obj().with("ok", v),
                Err(e) => JValue::obj().with("error", JValue::Str(e.to_string())),
            },
            Err(e) => JValue::obj().with("error", JValue::Str(e.to_string())),
        };
        resp.to_compact()
    }

    /// 요청 하나 처리.
    pub fn handle(&mut self, req: &JValue) -> Result<JValue> {
        let method = req.get("method").and_then(JValue::as_str).ok_or_else(|| CoreError::InvalidArgs("method가 없습니다".into()))?;
        let text = || req.get("text").and_then(JValue::as_str).unwrap_or("");
        let warnings_json = |w: Vec<crate::Warning>| w.to_json(true);
        match method {
            "new" => {
                self.doc = Document::new();
                Ok(JValue::Null)
            }
            "openKgbone" => {
                let (doc, w) = Document::open_kgbone(text().as_bytes())?;
                self.doc = doc;
                Ok(JValue::obj().with("warnings", warnings_json(w)))
            }
            "saveKgbone" => {
                let bytes = self.doc.save_kgbone()?;
                Ok(JValue::obj().with("text", JValue::Str(String::from_utf8_lossy(&bytes).into_owned())))
            }
            "importSpineJson" => {
                let (doc, w) = Document::import_spine_json(text().as_bytes())?;
                self.doc = doc;
                Ok(JValue::obj().with("warnings", warnings_json(w)))
            }
            "exportSpineJson" => {
                let opts = export_options(req.get("options"));
                let r = self.doc.export_spine_json(&opts)?;
                Ok(JValue::obj()
                    .with("text", JValue::Str(String::from_utf8_lossy(&r.bytes).into_owned()))
                    .with("warnings", warnings_json(r.warnings)))
            }
            "execute" => {
                let id = req.get("id").and_then(JValue::as_str).ok_or_else(|| CoreError::InvalidArgs("id가 없습니다".into()))?;
                let empty = JValue::obj();
                let args = req.get("args").unwrap_or(&empty);
                self.doc.execute(id, args)
            }
            "project" => Ok(self.doc.project_json()),
            "state" => Ok(JValue::obj()
                .with("dirty", JValue::Bool(self.doc.is_dirty()))
                .with("canUndo", JValue::Bool(self.doc.can_undo()))
                .with("canRedo", JValue::Bool(self.doc.can_redo()))
                .with("undoLabel", self.doc.undo_label().map(String::from).to_json(true))
                .with("redoLabel", self.doc.redo_label().map(String::from).to_json(true))),
            "pose" => {
                let r = pose_request(req.get("request"))?;
                Ok(self.doc.pose(&r).to_json(true))
            }
            "resetPhysics" => {
                self.doc.reset_physics();
                Ok(JValue::Null)
            }
            "evalExpr" => {
                let input = req.get("input").and_then(JValue::as_str).unwrap_or("");
                let current = req.get("current").and_then(JValue::as_f64).unwrap_or(0.0);
                Ok(JValue::num(crate::expr::eval(input, current)?))
            }
            "exportSpineBundle" => {
                let o = req.get("options");
                let opts = crate::export::SpineBundleOptions {
                    name: o.and_then(|o| o.get("name")).and_then(JValue::as_str).unwrap_or("skeleton").to_string(),
                    spine: export_options(o),
                    pack: crate::export::PackOptions::default(),
                };
                let b = crate::export::export_spine_bundle(&self.doc, self, &opts)?;
                Ok(bundle_json(b))
            }
            "exportPngSequence" => {
                let o = req.get("options");
                let get_f = |k: &str| o.and_then(|o| o.get(k)).and_then(JValue::as_f64);
                let mut opts = crate::export::PngSequenceOptions::default();
                opts.animation = o.and_then(|o| o.get("animation")).and_then(JValue::as_str).map(String::from);
                opts.skin = o.and_then(|o| o.get("skin")).and_then(JValue::as_str).map(String::from);
                if let Some(n) = o.and_then(|o| o.get("name")).and_then(JValue::as_str) {
                    opts.name = n.into();
                }
                opts.fps = get_f("fps").unwrap_or(30.0) as f32;
                opts.scale = get_f("scale").unwrap_or(1.0) as f32;
                opts.width = get_f("width").unwrap_or(0.0) as u32;
                opts.height = get_f("height").unwrap_or(0.0) as u32;
                let mut doc = std::mem::take(&mut self.doc);
                let r = crate::export::export_png_sequence(&mut doc, self, &opts);
                self.doc = doc;
                Ok(bundle_json(r?))
            }
            other => Err(CoreError::UnknownCommand(other.into())),
        }
    }

    /// 내보내기용 PNG 원본 등록.
    pub fn add_png(&mut self, path: &str, bytes: &[u8]) {
        self.pngs.insert(path.to_string(), bytes.to_vec());
    }

    /// 이미지 정보 등록(알파는 행 우선 0~255, 없으면 빈 배열).
    pub fn set_image(&mut self, path: &str, width: u32, height: u32, alpha: &[u8]) {
        let alpha = if alpha.is_empty() { None } else { Some(alpha.to_vec()) };
        self.doc.set_image(path, width, height, alpha);
    }
}

/// `{"version": "4.3"|"4.2", "pretty": bool, "imagesPath": string}`
pub fn export_options(v: Option<&JValue>) -> SpineExportOptions {
    let mut o = SpineExportOptions::default();
    if let Some(v) = v {
        if v.get("version").and_then(JValue::as_str) == Some("4.2") {
            o.version = SpineVersion::V42;
        }
        if let Some(p) = v.get("pretty").and_then(JValue::as_bool) {
            o.pretty = p;
        }
        if let Some(p) = v.get("imagesPath").and_then(JValue::as_str) {
            o.images_path = p.to_string();
        }
    }
    o
}

/// `{"animation"?, "time", "lastTime"?, "skin"?, "physics": "none"|"reset"|숫자(dt), "looping"}`
pub fn pose_request(v: Option<&JValue>) -> Result<PoseRequest> {
    let mut r = PoseRequest::default();
    let Some(v) = v else { return Ok(r) };
    r.animation = v.get("animation").and_then(JValue::as_str).map(String::from);
    r.time = v.get("time").map(f32::from_json).transpose()?.unwrap_or(0.0);
    r.last_time = v.get("lastTime").filter(|x| !x.is_null()).map(f32::from_json).transpose()?;
    r.skin = v.get("skin").and_then(JValue::as_str).map(String::from);
    r.looping = v.get("looping").and_then(JValue::as_bool).unwrap_or(false);
    r.physics = match v.get("physics") {
        Some(JValue::Str(s)) if s == "reset" => PhysicsStep::Reset,
        Some(x @ JValue::Num(_)) => PhysicsStep::Advance(f32::from_json(x)?),
        _ => PhysicsStep::None,
    };
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_method_is_error_text() {
        let mut s = Session::new();
        let r = s.handle_text(r#"{"method": "nope"}"#);
        assert!(r.contains("\"error\""), "{r}");
        let r = s.handle_text("{bad");
        assert!(r.contains("\"error\""), "{r}");
    }

    #[test]
    fn project_and_state() {
        let mut s = Session::new();
        let p = s.handle_text(r#"{"method": "project"}"#);
        assert!(p.contains("\"root\""), "{p}");
        let st = s.handle_text(r#"{"method": "state"}"#);
        assert!(st.contains("\"dirty\":false"), "{st}");
    }
}
