//! 뼈대 전체와 프로젝트(.kgbone 한 파일 = Project 하나).
use super::{Animation, Bone, Constraint, EventDef, Skin, Slot};
use crate::{json_object, json_string_enum};

#[derive(Clone, Debug, PartialEq)]
pub struct Skeleton {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub fps: f32,
    pub reference_scale: f32,
    pub bones: Vec<Bone>,
    pub slots: Vec<Slot>,
    pub constraints: Vec<Constraint>,
    pub skins: Vec<Skin>,
    pub events: Vec<EventDef>,
    pub animations: Vec<Animation>,
}

impl Default for Skeleton {
    fn default() -> Self {
        Skeleton {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            fps: 30.0,
            reference_scale: 100.0,
            bones: vec![],
            slots: vec![],
            constraints: vec![],
            skins: vec![],
            events: vec![],
            animations: vec![],
        }
    }
}

impl Skeleton {
    /// 루트 본 하나와 기본 스킨을 가진 새 뼈대.
    pub fn new_with_root() -> Self {
        Skeleton { bones: vec![Bone::new("root", None)], skins: vec![Skin::new(Skin::DEFAULT)], ..Default::default() }
    }

    pub fn bone_index(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|b| b.name == name)
    }
    pub fn bone(&self, name: &str) -> Option<&Bone> {
        self.bones.iter().find(|b| b.name == name)
    }
    pub fn bone_mut(&mut self, name: &str) -> Option<&mut Bone> {
        self.bones.iter_mut().find(|b| b.name == name)
    }
    pub fn slot_index(&self, name: &str) -> Option<usize> {
        self.slots.iter().position(|s| s.name == name)
    }
    pub fn slot(&self, name: &str) -> Option<&Slot> {
        self.slots.iter().find(|s| s.name == name)
    }
    pub fn slot_mut(&mut self, name: &str) -> Option<&mut Slot> {
        self.slots.iter_mut().find(|s| s.name == name)
    }
    pub fn constraint_index(&self, name: &str) -> Option<usize> {
        self.constraints.iter().position(|c| c.name() == name)
    }
    pub fn constraint(&self, name: &str) -> Option<&Constraint> {
        self.constraints.iter().find(|c| c.name() == name)
    }
    pub fn constraint_mut(&mut self, name: &str) -> Option<&mut Constraint> {
        self.constraints.iter_mut().find(|c| c.name() == name)
    }
    pub fn skin(&self, name: &str) -> Option<&Skin> {
        self.skins.iter().find(|s| s.name == name)
    }
    pub fn skin_mut(&mut self, name: &str) -> Option<&mut Skin> {
        self.skins.iter_mut().find(|s| s.name == name)
    }
    /// 기본 스킨(없으면 만든다).
    pub fn default_skin_mut(&mut self) -> &mut Skin {
        if self.skin(Skin::DEFAULT).is_none() {
            self.skins.insert(0, Skin::new(Skin::DEFAULT));
        }
        self.skin_mut(Skin::DEFAULT).expect("기본 스킨")
    }
    pub fn animation(&self, name: &str) -> Option<&Animation> {
        self.animations.iter().find(|a| a.name == name)
    }
    pub fn animation_mut(&mut self, name: &str) -> Option<&mut Animation> {
        self.animations.iter_mut().find(|a| a.name == name)
    }
    pub fn event(&self, name: &str) -> Option<&EventDef> {
        self.events.iter().find(|e| e.name == name)
    }

    /// 본 `name`이 `ancestor`의 자손(자기 자신 포함)인가.
    pub fn is_descendant(&self, name: &str, ancestor: &str) -> bool {
        let mut cur = Some(name.to_string());
        while let Some(n) = cur {
            if n == ancestor {
                return true;
            }
            cur = self.bone(&n).and_then(|b| b.parent.clone());
        }
        false
    }

    /// `base`로 시작하는, 아직 안 쓰인 이름(`base`, `base2`, `base3`…).
    pub fn unique_name(&self, base: &str, taken: impl Fn(&str) -> bool) -> String {
        if !taken(base) {
            return base.to_string();
        }
        (2..).map(|i| format!("{base}{i}")).find(|n| !taken(n)).expect("이름")
    }
}

/// 에디터 작업 상태(뼈대 결과에는 영향 없음).
#[derive(Clone, Debug, PartialEq)]
pub struct EditorState {
    pub mode: EditorMode,
    pub view: ViewState,
    pub active_animation: Option<String>,
    pub active_skin: Option<String>,
    pub time: f32,
    pub tree_expanded: Vec<String>,
    pub onion: OnionState,
    pub playback: PlaybackState,
}

impl Default for EditorState {
    fn default() -> Self {
        EditorState {
            mode: EditorMode::Setup,
            view: ViewState::default(),
            active_animation: None,
            active_skin: None,
            time: 0.0,
            tree_expanded: vec![],
            onion: OnionState::default(),
            playback: PlaybackState::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EditorMode {
    #[default]
    Setup,
    Animate,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewState {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}
impl Default for ViewState {
    fn default() -> Self {
        ViewState { x: 0.0, y: 0.0, zoom: 1.0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OnionState {
    pub enabled: bool,
    pub before: u32,
    pub after: u32,
    pub step: u32,
}
impl Default for OnionState {
    fn default() -> Self {
        OnionState { enabled: false, before: 2, after: 2, step: 1 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaybackState {
    pub speed: f32,
    pub loop_: bool,
}
impl Default for PlaybackState {
    fn default() -> Self {
        PlaybackState { speed: 1.0, loop_: true }
    }
}

/// 프로젝트 = .kgbone 파일 하나의 내용.
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub format_version: u32,
    pub generator: String,
    pub images_path: String,
    pub skeleton: Skeleton,
    pub editor: EditorState,
}
fn images_path() -> String {
    "./images".into()
}

impl Project {
    pub const FORMAT_VERSION: u32 = 1;
    pub const GENERATOR: &'static str = concat!("KGBone ", env!("CARGO_PKG_VERSION"));

    pub fn new() -> Self {
        Project {
            format_version: Self::FORMAT_VERSION,
            generator: Self::GENERATOR.into(),
            images_path: images_path(),
            skeleton: Skeleton::new_with_root(),
            editor: EditorState::default(),
        }
    }
}

impl Default for Project {
    fn default() -> Self {
        Project::new()
    }
}

json_object!(Skeleton {
    x: "x" = 0.0,
    y: "y" = 0.0,
    width: "width" = 0.0,
    height: "height" = 0.0,
    fps: "fps" = 30.0,
    reference_scale: "referenceScale" = 100.0,
    bones: "bones" = Vec::new(),
    slots: "slots" = Vec::new(),
    constraints: "constraints" = Vec::new(),
    skins: "skins" = Vec::new(),
    events: "events" = Vec::new(),
    animations: "animations" = Vec::new(),
});

json_string_enum!(EditorMode { Setup = "setup", Animate = "animate" });

json_object!(ViewState { x: "x" = 0.0, y: "y" = 0.0, zoom: "zoom" = 1.0 });
json_object!(OnionState { enabled: "enabled" = false, before: "before" = 2, after: "after" = 2, step: "step" = 1 });
json_object!(PlaybackState { speed: "speed" = 1.0, loop_: "loop" = true });

json_object!(EditorState {
    mode: "mode" = EditorMode::Setup,
    view: "view" = ViewState::default(),
    active_animation: "activeAnimation" = None,
    active_skin: "activeSkin" = None,
    time: "time" = 0.0,
    tree_expanded: "treeExpanded" = Vec::new(),
    onion: "onion" = OnionState::default(),
    playback: "playback" = PlaybackState::default(),
});

json_object!(Project {
    format_version: "formatVersion" = 0,
    generator: "generator" = String::new(),
    images_path: "imagesPath" = images_path(),
    skeleton: "skeleton" = Skeleton::default(),
    editor: "editor" = EditorState::default(),
});
