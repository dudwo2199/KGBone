//! 열린 문서 하나: 모델 + 되돌리기 기록 + 원본 보존 정보 + 포즈 계산기.
//!
//! 앱 본체와 WebAssembly는 이 타입만 들고 있으면 된다. 바깥에서 모델을 직접 바꾸는 길은 없고,
//! 모든 편집은 [`Document::execute`](명령 id + JSON 인자)로만 한다.
use crate::commands::{self, CommandContext, ImageStore};
use crate::format::kgbone::KgboneFormat;
use crate::format::spine_json::{SpineExportOptions, SpineJsonFormat};
use crate::format::{Original, SkeletonFormat, WriteResult};
use crate::history::{History, State};
use crate::json::JValue;
use crate::model::Project;
use crate::pose::{PoseEngine, PoseOutput, PoseRequest};
use crate::{CoreError, Result, Warning};
use crate::json::{JValue as Value, ToJson};

pub struct Document {
    project: Project,
    history: History,
    /// .kgbone에서 열었을 때의 원본(바이트 동일 저장·부분 보존용).
    kgbone_original: Option<Original>,
    /// Spine JSON에서 가져왔을 때의 원본(내보낼 때 모르는 필드·키 순서·숫자 표기 보존용).
    spine_original: Option<Original>,
    /// 마지막으로 저장(또는 연) 시점의 상태. 이것과 다르면 "저장하지 않은 변경".
    saved: State,
    engine: PoseEngine,
    images: ImageStore,
}

impl Default for Document {
    fn default() -> Self {
        Document::new()
    }
}

impl Document {
    /// 루트 본 하나가 있는 새 문서.
    pub fn new() -> Self {
        let project = Project::new();
        let saved = State::of(&project);
        Document {
            project,
            history: History::new(),
            kgbone_original: None,
            spine_original: None,
            saved,
            engine: PoseEngine::new(),
            images: ImageStore::default(),
        }
    }

    fn from_project(project: Project) -> Self {
        let saved = State::of(&project);
        Document { project, saved, ..Document::new() }
    }

    /// .kgbone 바이트로 연다.
    pub fn open_kgbone(bytes: &[u8]) -> Result<(Self, Vec<Warning>)> {
        let r = KgboneFormat.read(bytes)?;
        // 구버전을 변환해 열었으면 원래 바이트로 저장하면 안 되므로 바이트를 비운다.
        let bytes = if crate::format::kgbone::was_migrated(&r) { Vec::new() } else { bytes.to_vec() };
        let mut doc = Document::from_project(r.project.clone());
        doc.kgbone_original = Some(Original { bytes, raw: r.raw, project: r.project });
        Ok((doc, r.warnings))
    }

    /// .kgbone 바이트를 만든다. 편집하지 않았으면 연 파일과 바이트 단위로 같다. 저장 후에는 "변경 없음" 상태가 된다.
    pub fn save_kgbone(&mut self) -> Result<Vec<u8>> {
        let w = KgboneFormat.write(&self.project, self.kgbone_original.as_ref())?;
        let raw = crate::json::parse(std::str::from_utf8(&w.bytes).map_err(|e| CoreError::Parse(e.to_string()))?)?;
        self.kgbone_original = Some(Original { bytes: w.bytes.clone(), raw, project: self.project.clone() });
        self.saved = State::of(&self.project);
        Ok(w.bytes)
    }

    /// Spine JSON 호환 파일을 가져와 새 문서를 만든다(4.3 우선, 4.2 읽기).
    pub fn import_spine_json(bytes: &[u8]) -> Result<(Self, Vec<Warning>)> {
        let r = SpineJsonFormat::default().read(bytes)?;
        let mut doc = Document::from_project(r.project.clone());
        doc.spine_original = Some(Original { bytes: bytes.to_vec(), raw: r.raw, project: r.project });
        // 가져온 문서는 아직 .kgbone으로 저장되지 않았다.
        doc.saved = State { images_path: String::from("\u{0}"), ..State::of(&doc.project) };
        Ok((doc, r.warnings))
    }

    /// Spine JSON 호환 텍스트를 만든다. 가져온 문서면 원본의 모르는 필드·키 순서·숫자 표기를 보존한다.
    pub fn export_spine_json(&self, options: &SpineExportOptions) -> Result<WriteResult> {
        SpineJsonFormat { options: options.clone() }.write(&self.project, self.spine_original.as_ref())
    }

    /// 명령 실행. `edit.undo`, `edit.redo`, `history.begin`, `history.end`도 여기서 처리한다.
    /// 돌려주는 값은 명령마다 다르다(예: `bone.create` → `{"name": "bone2"}`).
    pub fn execute(&mut self, id: &str, args: &Value) -> Result<Value> {
        match id {
            "edit.undo" => return Ok(Value::obj().with("label", self.history.undo(&mut self.project).to_json(true))),
            "edit.redo" => return Ok(Value::obj().with("label", self.history.redo(&mut self.project).to_json(true))),
            "history.begin" => {
                let label = args.get("label").and_then(Value::as_str).unwrap_or("편집");
                self.history.begin_group(label, &self.project);
                return Ok(Value::Null);
            }
            "history.end" => {
                self.history.end_group(&self.project);
                return Ok(Value::Null);
            }
            _ => {}
        }
        let cmd = commands::find(id).ok_or_else(|| CoreError::UnknownCommand(id.into()))?;
        let before = State::of(&self.project);
        let result = {
            let mut ctx = CommandContext { project: &mut self.project, images: &self.images };
            cmd.run(&mut ctx, args)
        };
        match result {
            Ok(v) => {
                if cmd.undoable() {
                    self.history.record(cmd.label(), before, &self.project);
                }
                Ok(v)
            }
            Err(e) => {
                // 실패한 명령이 남긴 중간 변경은 되돌린다.
                self.project.skeleton = before.skeleton;
                self.project.images_path = before.images_path;
                Err(e)
            }
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    /// 화면에 보낼 모델 JSON(기본값 생략 없이 전체).
    pub fn project_json(&self) -> Value {
        self.project.to_json(true)
    }

    /// 저장하지 않은 뼈대 변경이 있는가(작업 상태 변경은 제외).
    pub fn is_dirty(&self) -> bool {
        State::of(&self.project) != self.saved
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
    pub fn undo_label(&self) -> Option<&str> {
        self.history.undo_label()
    }
    pub fn redo_label(&self) -> Option<&str> {
        self.history.redo_label()
    }

    /// 포즈 계산(미리보기·내보내기 공용).
    pub fn pose(&mut self, req: &PoseRequest) -> PoseOutput {
        self.engine.compute(&self.project.skeleton, req)
    }

    pub fn reset_physics(&mut self) {
        self.engine.reset_physics();
    }

    /// 이미지 크기·알파 정보 등록(이미지 폴더를 읽은 쪽이 알려 준다). 메시 자동 생성·이미지 첨부물 기본 크기에 쓴다.
    pub fn set_image(&mut self, path: &str, width: u32, height: u32, alpha: Option<Vec<u8>>) {
        self.images.set(path, width, height, alpha);
    }

    pub fn clear_images(&mut self) {
        self.images = ImageStore::default();
    }

    /// Spine 원본 트리(가져온 문서일 때).
    pub fn spine_raw(&self) -> Option<&JValue> {
        self.spine_original.as_ref().map(|o| &o.raw)
    }
}
