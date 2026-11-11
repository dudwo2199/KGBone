//! 뼈대 계산: 본 월드 변환, 제약(IK·트랜스폼·패스·물리·슬라이더), 메시 변형, 클리핑 → 그리기 목록.
//!
//! 바깥에서는 [`PoseEngine::compute`] 하나로 쓴다. 제약 종류별 계산은 `ConstraintSolver` trait으로 묶는다.
mod bone_transform;
mod clipping;
mod ik;
mod mesh_deform;
mod path;
mod physics;
mod slider;
mod transform;

use crate::model::{BlendMode, Skeleton, Transform};
use crate::json_object;

pub use bone_transform::{compute_world, world_to_local_transform, WorldBone};

/// 물리 진행 방법.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PhysicsStep {
    /// 물리 계산 없이 현재 상태 유지.
    None,
    /// 물리 상태를 처음으로 되돌림.
    Reset,
    /// 실제 시간 `dt`초만큼 진행(내부는 제약의 fps 고정 간격으로 나눠 계산).
    Advance(f32),
}

/// 포즈 요청.
#[derive(Clone, Debug, PartialEq)]
pub struct PoseRequest {
    /// 적용할 애니메이션(없으면 셋업 포즈).
    pub animation: Option<String>,
    /// 애니메이션 시점(초).
    pub time: f32,
    /// 이전 시점(이벤트 발생 구간 [last_time, time) 계산용). 없으면 이벤트 없음.
    pub last_time: Option<f32>,
    /// 켤 스킨(없으면 기본 스킨만).
    pub skin: Option<String>,
    pub physics: PhysicsStep,
    /// 반복 재생 중이면 길이로 나눈 나머지 시점을 쓴다.
    pub looping: bool,
}

impl Default for PoseRequest {
    fn default() -> Self {
        PoseRequest { animation: None, time: 0.0, last_time: None, skin: None, physics: PhysicsStep::None, looping: false }
    }
}

/// 그릴 삼각형 묶음 하나(슬롯 하나의 이미지/메시, 클리핑 적용 후).
#[derive(Clone, Debug, PartialEq)]
pub struct DrawItem {
    pub slot: String,
    pub attachment: String,
    /// 이미지 경로(확장자 없음, imagesPath 기준).
    pub image: String,
    pub blend: BlendMode,
    /// 슬롯 색 × 첨부물 색 (0~1).
    pub color: [f32; 4],
    /// 어두운 색(tint black), 없으면 None.
    pub dark: Option<[f32; 3]>,
    /// 월드 좌표 [x0, y0, x1, y1, …].
    pub positions: Vec<f32>,
    /// 텍스처 좌표 [u0, v0, …] (이미지 한 장 기준 0~1, v는 아래로 증가).
    pub uvs: Vec<f32>,
    pub indices: Vec<u32>,
}

/// 에디터 표시용 도형(바운딩 박스, 클리핑, 패스, 포인트, 메시 외곽선).
#[derive(Clone, Debug, PartialEq)]
pub struct Overlay {
    /// `boundingbox`, `clipping`, `path`, `point`, `mesh`.
    pub kind: String,
    pub slot: String,
    pub attachment: String,
    /// 월드 좌표 점들. path는 베지어 제어점 그대로, point는 [x, y, 방향x, 방향y], mesh는 정점 전체.
    pub points: Vec<f32>,
    /// mesh일 때 삼각형 번호(그 밖에는 빈 배열).
    pub indices: Vec<u32>,
    pub closed: bool,
    pub color: [f32; 4],
}

/// 구간 안에서 발생한 이벤트.
#[derive(Clone, Debug, PartialEq)]
pub struct FiredEvent {
    pub name: String,
    pub time: f32,
    pub int: i32,
    pub float: f32,
    pub string: String,
    pub volume: f32,
    pub balance: f32,
}

/// 계산 결과.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PoseOutput {
    /// `skeleton.bones` 순서의 월드 변환.
    pub bones: Vec<WorldBone>,
    /// 애니메이션·제약 적용 후의 로컬 값(`key.constrained`가 쓴다).
    pub locals: Vec<Transform>,
    /// 계산에 포함된 본인가(스킨 전용 본이 꺼져 있으면 false).
    pub active: Vec<bool>,
    /// 그리기 순서대로.
    pub draw: Vec<DrawItem>,
    pub overlays: Vec<Overlay>,
    pub events: Vec<FiredEvent>,
}

json_object!(DrawItem {
    slot: "slot" = String::new(),
    attachment: "attachment" = String::new(),
    image: "image" = String::new(),
    blend: "blend" = BlendMode::Normal,
    color: "color" = [1.0; 4],
    dark: "dark" = None,
    positions: "positions" = Vec::new(),
    uvs: "uvs" = Vec::new(),
    indices: "indices" = Vec::new(),
});
json_object!(Overlay {
    kind: "kind" = String::new(),
    slot: "slot" = String::new(),
    attachment: "attachment" = String::new(),
    points: "points" = Vec::new(),
    indices: "indices" = Vec::new(),
    closed: "closed" = false,
    color: "color" = [1.0; 4],
});
json_object!(FiredEvent {
    name: "name" = String::new(),
    time: "time" = 0.0,
    int: "int" = 0,
    float: "float" = 0.0,
    string: "string" = String::new(),
    volume: "volume" = 1.0,
    balance: "balance" = 0.0,
});
json_object!(PoseOutput {
    bones: "bones" = Vec::new(),
    locals: "locals" = Vec::new(),
    active: "active" = Vec::new(),
    draw: "draw" = Vec::new(),
    overlays: "overlays" = Vec::new(),
    events: "events" = Vec::new(),
});

/// 계산 중 상태. 애니메이션 적용(`anim`)이 이 값을 바꾸고 제약이 이어서 바꾼다.
#[derive(Clone, Debug, Default)]
pub struct SkeletonPose {
    /// 본마다 현재 로컬 값.
    pub locals: Vec<Transform>,
    /// 본마다 현재 상속 종류(애니메이션 inherit 키가 바꿀 수 있음).
    pub inherits: Vec<crate::model::Inherit>,
    pub world: Vec<WorldBone>,
    pub active: Vec<bool>,
    /// 슬롯마다 색, 어두운 색, 보이는 첨부물.
    pub slot_colors: Vec<[f32; 4]>,
    pub slot_darks: Vec<Option<[f32; 3]>>,
    pub slot_attachments: Vec<Option<String>>,
    /// 그리기 순서(슬롯 번호).
    pub draw_order: Vec<usize>,
    /// 제약마다 현재 값(제약 순서). 값 배열의 뜻은 FORMAT.md 3.7 표의 제약 타임라인과 같다.
    pub constraint_values: Vec<Vec<f32>>,
    /// (슬롯, 첨부물) → 셋업 정점에 더할 변형 값.
    pub deform: std::collections::HashMap<(String, String), Vec<f32>>,
    /// (슬롯, 첨부물) → 시퀀스 프레임 번호.
    pub sequence_index: std::collections::HashMap<(String, String), i32>,
}

impl SkeletonPose {
    /// 셋업 포즈로 초기화.
    pub fn setup(skel: &Skeleton) -> Self {
        let _ = skel;
        SkeletonPose::default()
    }
}

/// 제약 종류별 계산.
#[allow(dead_code)]
pub(crate) trait ConstraintSolver {
    /// `pose.world`가 앞선 계산까지 반영된 상태에서 `index`번 제약을 적용하고, 바뀐 본의 월드 값을 갱신한다.
    fn apply(&mut self, skel: &Skeleton, index: usize, pose: &mut SkeletonPose, ctx: &mut SolveContext);
}

/// 제약 계산에 필요한 공통 정보.
#[allow(dead_code)]
pub(crate) struct SolveContext<'a> {
    pub skin: Option<&'a str>,
    pub physics: PhysicsStep,
}

/// 물리 상태를 시간에 걸쳐 들고 있는 계산기. 같은 입력이면 항상 같은 결과(고정 간격).
#[derive(Default)]
pub struct PoseEngine {
    physics: physics::PhysicsState,
}

impl PoseEngine {
    pub fn new() -> Self {
        PoseEngine::default()
    }

    pub fn compute(&mut self, skel: &Skeleton, req: &PoseRequest) -> PoseOutput {
        let _ = (skel, req, &mut self.physics);
        PoseOutput::default()
    }

    pub fn reset_physics(&mut self) {
        self.physics = physics::PhysicsState::default();
    }
}

/// 셋업 포즈(제약 포함)의 월드 변환만 계산한다. 명령(부모 변경, 바인딩 등)이 쓴다.
pub fn setup_world(skel: &Skeleton) -> Vec<WorldBone> {
    // 제약 계산이 완성되면 PoseEngine::compute(셋업 포즈) 결과로 바꾼다.
    let locals: Vec<_> = skel.bones.iter().map(|b| b.local()).collect();
    let inherits: Vec<_> = skel.bones.iter().map(|b| b.inherit).collect();
    compute_world(skel, &locals, &inherits)
}
