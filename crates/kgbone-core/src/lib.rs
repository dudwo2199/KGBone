//! KGBone 핵심 모듈.
//!
//! 앱 본체(Tauri, 파일 처리)와 WebAssembly(화면 미리보기 계산)가 함께 쓰는 유일한 계산 코드.
//! - [`model`]: 공통 데이터 모델 (docs/FORMAT.md)
//! - [`json`]: 키 순서·숫자 표기를 보존하는 JSON 트리, 합치기, 출력
//! - [`format`]: .kgbone / Spine JSON / .atlas 읽기·쓰기 (`SkeletonFormat` trait)
//! - [`pose`]: 뼈대 계산(본 변환, 제약, 물리, 메시 변형, 클리핑) → 그리기 목록
//! - [`anim`]: 곡선 보간과 애니메이션 적용
//! - [`mesh`]: 삼각분할, 자동 메시, 가중치
//! - [`commands`]: 명령(id + JSON 인자) 실행 — `interface/commands.json`과 1:1
//! - [`history`]: 되돌리기 기록
//! - [`document`]: 열린 문서 하나(모델 + 기록 + 원본 보존 정보)
//! - [`export`]: 텍스처 패킹, PNG 시퀀스(소프트웨어 그리기)
//! - [`expr`]: 숫자 입력칸 수식
//! - [`api`]: 화면·앱 본체가 쓰는 JSON 요청 처리기(`interface/core-api.md`)
pub mod anim;
pub mod api;
pub mod commands;
pub mod document;
mod error;
pub mod export;
pub mod expr;
pub mod format;
pub mod history;
pub mod json;
pub mod mesh;
pub mod model;
pub mod pose;

pub use document::Document;
pub use error::{CoreError, Result, Warning};
