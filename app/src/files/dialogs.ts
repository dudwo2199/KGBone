// 파일 흐름이 쓰는 대화상자 인터페이스(화면의 DialogService가 구현, 단위 테스트는 가짜로 대신한다).
export type SaveChoice = 'save' | 'discard' | 'cancel';

export interface SpineExportChoice {
  dir: string;
  version: '4.3' | '4.2';
  pretty: boolean;
  maxPageSize: number;
  padding: number;
}

export interface PngSequenceChoice {
  dir: string;
  animation: string | null;
  fps: number;
  scale: number;
  width: number;
  height: number;
  transparent: boolean;
}

export interface AppDialogs {
  /** "저장하지 않은 변경이 있습니다. 저장할까요?" [저장][저장 안 함][취소] */
  confirmSave(): Promise<SaveChoice>;
  prompt(title: string, label: string, initial: string): Promise<string | null>;
  message(title: string, text: string): Promise<void>;
  /** 경고·결과 목록 */
  report(title: string, lines: string[], warnings: string[]): Promise<void>;
  exportSpine(defaults: SpineExportChoice): Promise<SpineExportChoice | null>;
  pngSequence(defaults: PngSequenceChoice, animations: string[]): Promise<PngSequenceChoice | null>;
  /** 이미지 고르기(이미지 첨부물 만들기) */
  pickImage(names: string[]): Promise<string | null>;
}
