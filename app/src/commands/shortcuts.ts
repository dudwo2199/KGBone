// 단축키: 문자열("Ctrl+Shift+N") 해석, 키 이벤트와 맞추기, 입력칸·한국어 조합 중 무시 규칙(I02, UI_SPEC 8장).
import { COMMAND_DEFS } from './registry';

export interface Combo {
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  /** 대문자 글자, 또는 키 이름(Delete, F2, Home, Escape, Tab…), 또는 기호(+ = - , .) */
  key: string;
}

export interface KeyLike {
  key: string;
  code?: string;
  ctrlKey: boolean;
  metaKey?: boolean;
  shiftKey: boolean;
  altKey: boolean;
  isComposing?: boolean;
  keyCode?: number;
}

export function parseShortcut(s: string): Combo {
  // "Ctrl+=" / "Ctrl+-" / "+" 처리: 마지막 '+' 뒤가 비면 키가 '+'
  const parts = s.split('+');
  let key = parts.pop() ?? '';
  if (key === '' && parts.length) {
    parts.pop();
    key = '+';
  }
  const mods = parts.map((p) => p.toLowerCase());
  return { ctrl: mods.includes('ctrl'), shift: mods.includes('shift'), alt: mods.includes('alt'), key: key.length === 1 ? key.toUpperCase() : key };
}

const isLetter = (k: string) => /^[A-Z]$/.test(k);

/** 키 이벤트의 키 이름(한글 자판이면 code로 글자를 얻는다) */
function eventKey(e: KeyLike): string {
  let k = e.key;
  if (k === 'Esc') k = 'Escape';
  if (k === 'Del') k = 'Delete';
  if (k.length === 1) {
    const up = k.toUpperCase();
    if (/^[A-Z0-9]$/.test(up)) return up;
    if (!/^[\x20-\x7e]$/.test(k) && e.code) {
      const m = /^Key([A-Z])$/.exec(e.code);
      if (m) return m[1];
    }
    return k;
  }
  return k;
}

export function matches(c: Combo, e: KeyLike): boolean {
  const ctrl = e.ctrlKey || !!e.metaKey;
  if (c.ctrl !== ctrl || c.alt !== e.altKey) return false;
  const k = eventKey(e);
  // 기호 키는 Shift를 따지지 않는다("+"는 보통 Shift+=)
  if (!isLetter(c.key) && c.key.length === 1) {
    if (c.shift && !e.shiftKey) return false;
    if (k === c.key) return true;
    // Ctrl+= 는 숫자 패드 + 로도
    return c.key === '=' && k === '+' && c.ctrl;
  }
  if (c.shift !== e.shiftKey) return false;
  return k === c.key;
}

export interface KeyContext {
  /** 초점이 글자 입력칸(input, textarea, contenteditable)에 있는가 */
  inTextField: boolean;
  /** compositionstart~compositionend 사이인가 */
  composing: boolean;
}

/** 이 키 이벤트를 명령으로 보내도 되는가(I02) */
export function shouldDispatch(e: KeyLike, ctx: KeyContext): boolean {
  if (ctx.composing || e.isComposing || e.keyCode === 229 || e.key === 'Process') return false;
  if (ctx.inTextField) {
    return e.ctrlKey || !!e.metaKey || e.altKey || e.key === 'Escape' || e.key === 'Enter';
  }
  return true;
}

export interface ShortcutEntry {
  id: string;
  combo: Combo;
  text: string;
}

/** commands.json의 모든 단축키 */
export function allShortcuts(): ShortcutEntry[] {
  const out: ShortcutEntry[] = [];
  for (const c of COMMAND_DEFS) for (const s of c.shortcut) out.push({ id: c.id, combo: parseShortcut(s), text: s });
  return out;
}

/** 키 이벤트 → 명령 id(없으면 null). 더 구체적인(수식키가 많은) 단축키를 먼저 본다 */
export function findCommand(e: KeyLike, list: ShortcutEntry[] = allShortcuts()): string | null {
  const sorted = [...list].sort((a, b) => weight(b.combo) - weight(a.combo));
  for (const s of sorted) if (matches(s.combo, e)) return s.id;
  return null;
}
function weight(c: Combo): number {
  return (c.ctrl ? 4 : 0) + (c.alt ? 2 : 0) + (c.shift ? 1 : 0);
}

/** 화면에 보일 단축키 글자 */
export function shortcutText(id: string): string {
  const c = COMMAND_DEFS.find((x) => x.id === id);
  if (!c || !c.shortcut.length) return '';
  return c.shortcut[0] === 'Escape' ? 'Esc' : c.shortcut[0];
}

/** 문서 전체 한국어 조합 상태 추적 */
export class CompositionTracker {
  composing = false;
  private onStart = () => {
    this.composing = true;
  };
  private onEnd = () => {
    this.composing = false;
  };
  attach(target: Document): () => void {
    target.addEventListener('compositionstart', this.onStart, true);
    target.addEventListener('compositionend', this.onEnd, true);
    return () => {
      target.removeEventListener('compositionstart', this.onStart, true);
      target.removeEventListener('compositionend', this.onEnd, true);
    };
  }
}

export function isTextField(el: Element | null): boolean {
  if (!el) return false;
  const tag = el.tagName;
  if (tag === 'TEXTAREA') return true;
  if (tag === 'SELECT') return true;
  if (tag === 'INPUT') {
    const t = (el as HTMLInputElement).type;
    return !['checkbox', 'radio', 'button', 'range', 'color'].includes(t);
  }
  return (el as HTMLElement).isContentEditable === true;
}
