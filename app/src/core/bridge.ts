// Rust 핵심(WebAssembly)과 화면 사이의 연결 인터페이스.
// 메서드 이름은 crates/kgbone-core/src/api.rs의 method 문자열과 1:1 (interface/core-api.md).
import type { Bundle, DocState, PoseOutput, PoseRequest, Project, Warning } from './types';

export interface SpineExportOptions {
  version?: '4.3' | '4.2';
  pretty?: boolean;
  imagesPath?: string;
  /** 묶음 내보내기 파일 이름 앞부분 */
  name?: string;
}

export interface PngSequenceOptions {
  name?: string;
  animation?: string | null;
  skin?: string | null;
  fps?: number;
  scale?: number;
  width?: number;
  height?: number;
}

/** 핵심 모듈 하나(문서 하나). 모든 편집은 execute로만 한다. */
export interface Core {
  newDocument(): void;
  openKgbone(text: string): { warnings: Warning[] };
  saveKgbone(): string;
  importSpineJson(text: string): { warnings: Warning[] };
  exportSpineJson(options?: SpineExportOptions): { text: string; warnings: Warning[] };
  exportSpineBundle(options?: SpineExportOptions): Bundle;
  exportPngSequence(options?: PngSequenceOptions): Bundle;
  /** 명령 실행(interface/commands.json의 scope=core). 실패하면 한국어 메시지로 Error를 던진다. */
  execute<T = unknown>(id: string, args?: Record<string, unknown>): T;
  /** 여러 명령을 되돌리기 한 번으로 묶는다(드래그 등). */
  beginGroup(label: string): void;
  endGroup(): void;
  project(): Project;
  state(): DocState;
  pose(request: PoseRequest): PoseOutput;
  resetPhysics(): void;
  /** 숫자 입력칸 수식: v/value, +=, -=, *=, /=, 사칙연산 */
  evalExpr(input: string, current: number): number;
  /** 이미지 크기·알파 등록(메시 자동 생성, 이미지 첨부물 기본 크기) */
  setImage(path: string, width: number, height: number, alpha?: Uint8Array): void;
  /** 내보내기용 PNG 원본 등록 */
  addPng(path: string, png: Uint8Array): void;
}

/** 핵심 오류(응답의 error). */
export class CoreError extends Error {}
