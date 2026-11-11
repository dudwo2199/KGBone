// 공통 데이터 모델 타입(화면 쪽). Rust `kgbone_core::model`을 전체(full) 모드로 직렬화한 모양과 같다.
// 필드 이름·뜻은 docs/FORMAT.md. 값이 없으면 null.

export type Inherit = 'normal' | 'onlyTranslation' | 'noRotationOrReflection' | 'noScale' | 'noScaleOrReflection';
export type BlendMode = 'normal' | 'additive' | 'multiply' | 'screen';
export type BoneProperty = 'rotate' | 'x' | 'y' | 'scaleX' | 'scaleY' | 'shearY';
export type Hex = string; // "RRGGBBAA" 또는 "RRGGBB"

export interface Bone {
  name: string;
  parent: string | null;
  length: number;
  x: number;
  y: number;
  rotation: number;
  scaleX: number;
  scaleY: number;
  shearX: number;
  shearY: number;
  inherit: Inherit;
  skinRequired: boolean;
  color: Hex;
  icon: string | null;
  visible: boolean;
}

export interface Slot {
  name: string;
  bone: string;
  color: Hex;
  dark: Hex | null;
  attachment: string | null;
  blend: BlendMode;
  visible: boolean;
}

export interface Sequence {
  count: number;
  start: number;
  digits: number;
  setup: number;
}

export interface RegionAttachment {
  type: 'region';
  path: string | null;
  x: number;
  y: number;
  rotation: number;
  scaleX: number;
  scaleY: number;
  width: number;
  height: number;
  color: Hex;
  sequence: Sequence | null;
}
export interface MeshAttachment {
  type: 'mesh';
  path: string | null;
  width: number;
  height: number;
  color: Hex;
  uvs: number[];
  triangles: number[];
  weighted: boolean;
  vertices: number[];
  hull: number;
  edges: number[];
  sequence: Sequence | null;
}
export interface LinkedMeshAttachment {
  type: 'linkedmesh';
  path: string | null;
  width: number;
  height: number;
  color: Hex;
  parent: string;
  skin: string | null;
  timelines: boolean;
  sequence: Sequence | null;
}
export interface BoundingBoxAttachment {
  type: 'boundingbox';
  vertexCount: number;
  weighted: boolean;
  vertices: number[];
  color: Hex;
}
export interface PointAttachment {
  type: 'point';
  x: number;
  y: number;
  rotation: number;
  color: Hex;
}
export interface ClippingAttachment {
  type: 'clipping';
  end: string | null;
  vertexCount: number;
  weighted: boolean;
  vertices: number[];
  inverse: boolean;
  convex: boolean;
  color: Hex;
}
export interface PathAttachment {
  type: 'path';
  closed: boolean;
  constantSpeed: boolean;
  vertexCount: number;
  weighted: boolean;
  vertices: number[];
  lengths: number[];
  color: Hex;
}
export type Attachment =
  | RegionAttachment
  | MeshAttachment
  | LinkedMeshAttachment
  | BoundingBoxAttachment
  | PointAttachment
  | ClippingAttachment
  | PathAttachment;
export type AttachmentType = Attachment['type'];

export interface IkConstraint {
  type: 'ik';
  name: string;
  skinRequired: boolean;
  bones: string[];
  target: string;
  mix: number;
  softness: number;
  bendPositive: boolean;
  compress: boolean;
  stretch: boolean;
  uniform: boolean;
  volume: boolean;
}
export interface ToProperty {
  property: BoneProperty;
  offset: number;
  max: number;
  scale: number;
}
export interface FromProperty {
  from: BoneProperty;
  offset: number;
  to: ToProperty[];
}
export interface TransformConstraint {
  type: 'transform';
  name: string;
  skinRequired: boolean;
  bones: string[];
  source: string;
  localSource: boolean;
  localTarget: boolean;
  additive: boolean;
  clamp: boolean;
  mixRotate: number;
  mixX: number;
  mixY: number;
  mixScaleX: number;
  mixScaleY: number;
  mixShearY: number;
  properties: FromProperty[];
}
export interface PathConstraint {
  type: 'path';
  name: string;
  skinRequired: boolean;
  bones: string[];
  target: string;
  positionMode: 'fixed' | 'percent';
  spacingMode: 'length' | 'fixed' | 'percent' | 'proportional';
  rotateMode: 'tangent' | 'chain' | 'chainScale';
  offsetRotation: number;
  position: number;
  spacing: number;
  mixRotate: number;
  mixX: number;
  mixY: number;
}
export interface PhysicsConstraint {
  type: 'physics';
  name: string;
  skinRequired: boolean;
  bone: string;
  x: number;
  y: number;
  rotate: number;
  scaleX: number;
  shearX: number;
  limit: number;
  fps: number;
  inertia: number;
  strength: number;
  damping: number;
  mass: number;
  wind: number;
  gravity: number;
  mix: number;
  volume: boolean;
}
export interface SliderConstraint {
  type: 'slider';
  name: string;
  skinRequired: boolean;
  animation: string;
  time: number;
  loop: boolean;
  additive: boolean;
  mix: number;
  bone: string | null;
  property: BoneProperty;
  propertyFrom: number;
  propertyTo: number;
  local: boolean;
}
export type Constraint = IkConstraint | TransformConstraint | PathConstraint | PhysicsConstraint | SliderConstraint;
export type ConstraintType = Constraint['type'];

export interface Skin {
  name: string;
  bones: string[];
  constraints: string[];
  /** 슬롯 이름 → 첨부물 이름 → 첨부물 */
  attachments: Record<string, Record<string, Attachment>>;
}

export interface EventDef {
  name: string;
  int: number;
  float: number;
  string: string;
  audio: string;
  volume: number;
  balance: number;
}

/** 곡선: null = 선형, "stepped" = 계단, 숫자 배열 = 베지어(채널마다 cx1, cy1, cx2, cy2, 절대 좌표) */
export type Curve = null | 'stepped' | number[];

export interface Key {
  time: number;
  v: number[];
  s: string | null;
  offset: number;
  curve: Curve;
}
export type Timelines = Record<string, Key[]>;

export interface DrawOrderKey {
  time: number;
  offsets: { slot: string; offset: number }[];
}
export interface EventKey {
  time: number;
  name: string;
  int: number | null;
  float: number | null;
  string: string | null;
  volume: number | null;
  balance: number | null;
}
export interface Animation {
  name: string;
  bones: Record<string, Timelines>;
  slots: Record<string, Timelines>;
  constraints: Record<string, Timelines>;
  deform: Record<string, Record<string, Record<string, Key[]>>>;
  sequence: Record<string, Record<string, Record<string, Key[]>>>;
  drawOrder: DrawOrderKey[];
  events: EventKey[];
}

export interface Skeleton {
  x: number;
  y: number;
  width: number;
  height: number;
  fps: number;
  referenceScale: number;
  bones: Bone[];
  slots: Slot[];
  constraints: Constraint[];
  skins: Skin[];
  events: EventDef[];
  animations: Animation[];
}

export type EditorMode = 'setup' | 'animate';
export interface EditorState {
  mode: EditorMode;
  view: { x: number; y: number; zoom: number };
  activeAnimation: string | null;
  activeSkin: string | null;
  time: number;
  treeExpanded: string[];
  onion: { enabled: boolean; before: number; after: number; step: number };
  playback: { speed: number; loop: boolean };
}

export interface Project {
  formatVersion: number;
  generator: string;
  imagesPath: string;
  skeleton: Skeleton;
  editor: EditorState;
}

// ---------- 포즈 계산 결과 (kgbone_core::pose::PoseOutput) ----------
export interface WorldBone {
  a: number;
  b: number;
  c: number;
  d: number;
  x: number;
  y: number;
}
export interface DrawItem {
  slot: string;
  attachment: string;
  /** 이미지 경로(확장자 없음, imagesPath 기준) */
  image: string;
  blend: BlendMode;
  color: [number, number, number, number];
  dark: [number, number, number] | null;
  /** 월드 좌표 [x0, y0, ...] */
  positions: number[];
  /** 0~1, v는 아래로 */
  uvs: number[];
  indices: number[];
}
export interface Overlay {
  kind: 'boundingbox' | 'clipping' | 'path' | 'point' | 'mesh';
  slot: string;
  attachment: string;
  points: number[];
  indices: number[];
  closed: boolean;
  color: [number, number, number, number];
}
export interface FiredEvent {
  name: string;
  time: number;
  int: number;
  float: number;
  string: string;
  volume: number;
  balance: number;
}
export interface PoseOutput {
  bones: WorldBone[];
  locals: {
    x: number;
    y: number;
    rotation: number;
    scaleX: number;
    scaleY: number;
    shearX: number;
    shearY: number;
  }[];
  active: boolean[];
  draw: DrawItem[];
  overlays: Overlay[];
  events: FiredEvent[];
}
export interface PoseRequest {
  animation?: string | null;
  time: number;
  lastTime?: number | null;
  skin?: string | null;
  /** 'none' | 'reset' | 진행할 초(dt) */
  physics?: 'none' | 'reset' | number;
  looping?: boolean;
}

export interface Warning {
  code: string;
  message: string;
  path: string;
}

export interface DocState {
  dirty: boolean;
  canUndo: boolean;
  canRedo: boolean;
  undoLabel: string | null;
  redoLabel: string | null;
}

/** 내보내기 결과: 파일 이름과 base64 내용 */
export interface BundleFile {
  name: string;
  base64: string;
}
export interface Bundle {
  files: BundleFile[];
  warnings: Warning[];
}
