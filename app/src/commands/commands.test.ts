import { describe, expect, it } from 'vitest';
import { buildRegistry } from './handlers';
import { menuCommandIds } from './menus';
import { COMMAND_DEFS } from './registry';
import { allShortcuts, findCommand, parseShortcut, shouldDispatch, type KeyLike } from './shortcuts';

const key = (k: Partial<KeyLike> & { key: string }): KeyLike => ({ ctrlKey: false, shiftKey: false, altKey: false, ...k });

describe('L02 명령 등록', () => {
  const reg = buildRegistry();

  it('u_l02_all_shortcuts_point_to_registered_handlers', () => {
    const list = allShortcuts();
    expect(list.length).toBeGreaterThan(30);
    for (const s of list) expect(reg.has(s.id), `처리기 없음: ${s.id} (${s.text})`).toBe(true);
  });

  it('u_l02_every_ui_and_platform_command_has_handler', () => {
    for (const c of COMMAND_DEFS) if (c.scope !== 'core') expect(reg.hasOwn(c.id), `처리기 없음: ${c.id}`).toBe(true);
  });

  it('u_l02_menu_items_are_known_commands', () => {
    const ids = new Set(COMMAND_DEFS.map((c) => c.id));
    for (const id of menuCommandIds()) expect(ids.has(id), id).toBe(true);
  });

  it('u_l02_shortcut_keys_map_to_command_ids', () => {
    expect(findCommand(key({ key: 'N', ctrlKey: true, shiftKey: true }))).toBe('bone.create');
    expect(findCommand(key({ key: 'n', ctrlKey: true }))).toBe('file.new');
    expect(findCommand(key({ key: 'k' }))).toBe('key.edited');
    expect(findCommand(key({ key: 'K', shiftKey: true }))).toBe('key.constrained');
    expect(findCommand(key({ key: 'z', ctrlKey: true, shiftKey: true }))).toBe('edit.redo');
    expect(findCommand(key({ key: '+', shiftKey: true }))).toBe('slot.moveUp');
    expect(findCommand(key({ key: '=', ctrlKey: true }))).toBe('view.zoomIn');
    expect(findCommand(key({ key: ',', altKey: true }))).toBe('time.prevKey');
    expect(findCommand(key({ key: ',' }))).toBe('time.prevFrame');
    expect(findCommand(key({ key: 'Tab' }))).toBe('mode.toggle');
    expect(findCommand(key({ key: 'F2' }))).toBe('item.rename');
    // 한글 자판 상태: key는 자모, code로 글자를 찾는다
    expect(findCommand(key({ key: 'ㅏ', code: 'KeyK' }))).toBe('key.edited');
  });

  it('u_l02_parse_shortcut_symbols', () => {
    expect(parseShortcut('Ctrl+=')).toEqual({ ctrl: true, shift: false, alt: false, key: '=' });
    expect(parseShortcut('+')).toEqual({ ctrl: false, shift: false, alt: false, key: '+' });
    expect(parseShortcut('Ctrl+Shift+N').key).toBe('N');
  });
});

describe('I02 한국어 조합 중 단축키 무시', () => {
  it('u_i02_composing_key_is_not_dispatched', () => {
    const ctx = { inTextField: false, composing: false };
    expect(shouldDispatch(key({ key: 'k' }), ctx)).toBe(true);
    expect(shouldDispatch(key({ key: 'k', isComposing: true }), ctx)).toBe(false);
    expect(shouldDispatch(key({ key: 'Process', keyCode: 229 }), ctx)).toBe(false);
    expect(shouldDispatch(key({ key: 'k' }), { inTextField: false, composing: true })).toBe(false);
    // 조합 중이면 Ctrl 조합도 보내지 않는다
    expect(shouldDispatch(key({ key: 's', ctrlKey: true, isComposing: true }), ctx)).toBe(false);
  });

  it('u_i02_text_field_only_allows_ctrl_alt_esc_enter', () => {
    const ctx = { inTextField: true, composing: false };
    expect(shouldDispatch(key({ key: 'k' }), ctx)).toBe(false);
    expect(shouldDispatch(key({ key: 'Delete' }), ctx)).toBe(false);
    expect(shouldDispatch(key({ key: 's', ctrlKey: true }), ctx)).toBe(true);
    expect(shouldDispatch(key({ key: 'Escape' }), ctx)).toBe(true);
    expect(shouldDispatch(key({ key: 'Enter' }), ctx)).toBe(true);
  });
});
