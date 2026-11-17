// 메뉴 막대 구성(UI_SPEC 2장). 항목은 명령 id(+인자)만 들고, 글자·단축키는 commands.json에서 가져온다.
import { commandLabel } from './registry';
import { shortcutText } from './shortcuts';

export interface MenuCommand {
  type: 'command';
  id: string;
  args?: Record<string, unknown>;
  /** commands.json 라벨 대신 보일 글자(하위 메뉴 항목) */
  label?: string;
}
export interface MenuSub {
  type: 'sub';
  label: string;
  items: MenuItem[];
}
export type MenuItem = MenuCommand | MenuSub | { type: 'sep' };

export interface Menu {
  label: string;
  items: MenuItem[];
}

const c = (id: string, label?: string, args?: Record<string, unknown>): MenuCommand => ({ type: 'command', id, label, args });
const sep = { type: 'sep' } as const;

export const MENUS: Menu[] = [
  {
    label: '파일',
    items: [
      c('file.new'),
      c('file.open'),
      c('file.save'),
      c('file.saveAs'),
      sep,
      { type: 'sub', label: '가져오기', items: [c('file.importSpineJson', 'Spine JSON 호환(.json)…')] },
      {
        type: 'sub',
        label: '내보내기',
        items: [c('file.exportSpine', 'Spine JSON 호환 + 아틀라스…'), c('file.exportPngSequence', 'PNG 시퀀스…')],
      },
      sep,
      c('file.setImagesPath'),
      c('file.reloadImages'),
      c('file.registerAssociation'),
      sep,
      c('app.quit'),
    ],
  },
  {
    label: '편집',
    items: [
      c('edit.undo'),
      c('edit.redo'),
      sep,
      c('edit.delete'),
      c('item.rename'),
      c('edit.duplicate'),
      c('bone.setParentPick'),
      sep,
      c('slot.moveUp'),
      c('slot.moveDown'),
      sep,
      c('select.none', '전체 선택 해제'),
    ],
  },
  {
    label: '보기',
    items: [
      c('view.zoomIn'),
      c('view.zoomOut'),
      c('view.fit'),
      c('view.zoomReset'),
      sep,
      c('view.toggleBones'),
      c('view.toggleImages'),
      c('view.toggleOnion'),
      sep,
      c('view.showGraph'),
      c('view.showDopesheet'),
    ],
  },
  {
    label: '만들기',
    items: [
      c('bone.create'),
      c('slot.create'),
      {
        type: 'sub',
        label: '첨부물',
        items: [
          c('attachment.create', '이미지', { type: 'region' }),
          c('attachment.create', '메시', { type: 'mesh' }),
          c('attachment.create', '링크드 메시', { type: 'linkedmesh' }),
          c('attachment.create', '바운딩 박스', { type: 'boundingbox' }),
          c('attachment.create', '포인트', { type: 'point' }),
          c('attachment.create', '클리핑', { type: 'clipping' }),
          c('attachment.create', '패스', { type: 'path' }),
        ],
      },
      {
        type: 'sub',
        label: '제약',
        items: [
          c('constraint.create', 'IK', { type: 'ik' }),
          c('constraint.create', '트랜스폼', { type: 'transform' }),
          c('constraint.create', '패스', { type: 'path' }),
          c('constraint.create', '물리', { type: 'physics' }),
          c('constraint.create', '슬라이더', { type: 'slider' }),
        ],
      },
      sep,
      c('skin.create'),
      c('event.create'),
      c('animation.create'),
    ],
  },
  {
    label: '애니메이션',
    items: [
      c('mode.toggle'),
      sep,
      c('play.toggle'),
      c('play.reverse'),
      c('time.prevFrame'),
      c('time.nextFrame'),
      c('time.prevKey'),
      c('time.nextKey'),
      c('time.first'),
      c('time.last'),
      c('play.toggleLoop'),
      sep,
      c('key.edited'),
      c('key.constrained'),
      c('key.all'),
      c('edit.delete', '선택 키 삭제'),
      {
        type: 'sub',
        label: '보간',
        items: [c('key.setCurve', '선형', { curve: 'linear' }), c('key.setCurve', '계단', { curve: 'stepped' }), c('key.setCurve', '베지어', { curve: 'bezier' })],
      },
    ],
  },
  { label: '도구', items: [c('ai.connect')] },
  { label: '도움말', items: [c('help.shortcuts'), c('help.about')] },
];

export function menuItemLabel(m: MenuCommand): string {
  return m.label ?? commandLabel(m.id);
}
export function menuItemShortcut(m: MenuCommand): string {
  return m.args ? '' : shortcutText(m.id);
}

/** 메뉴에 나오는 모든 명령 id */
export function menuCommandIds(items: MenuItem[] = MENUS.flatMap((m) => m.items)): string[] {
  return items.flatMap((i) => (i.type === 'command' ? [i.id] : i.type === 'sub' ? menuCommandIds(i.items) : []));
}
