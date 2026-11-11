//! 공통 데이터 모델. 앱 본체·WebAssembly·내보내기가 모두 이 타입을 쓴다.
//! 필드 이름과 기본값은 docs/FORMAT.md와 1:1로 대응한다.
mod animation;
mod attachment;
mod bone;
mod color;
mod constraint;
mod omap;
mod skeleton;
mod skin;
mod slot;

pub use animation::{timeline, Animation, AttachmentKeys, Curve, DrawOrderKey, DrawOrderOffset, EventKey, Key, Timelines};
pub use attachment::{
    Attachment, BoundingBoxAttachment, ClippingAttachment, LinkedMeshAttachment, MeshAttachment, PathAttachment, PointAttachment,
    RegionAttachment, Sequence,
};
pub use bone::{Bone, Inherit, Transform};
pub use color::{Color, DarkColor};
pub use constraint::{
    BoneProperty, Constraint, FromProperty, IkConstraint, PathConstraint, PhysicsConstraint, PositionMode, RotateMode,
    SliderConstraint, SpacingMode, ToProperty, TransformConstraint,
};
pub use omap::OMap;
pub use skeleton::{EditorMode, EditorState, OnionState, PlaybackState, Project, Skeleton, ViewState};
pub use skin::{EventDef, Skin, SkinAttachments};
pub use slot::{BlendMode, Slot};
