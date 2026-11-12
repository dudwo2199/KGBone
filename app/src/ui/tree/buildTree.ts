// 트리 노드 만들기: 뼈대 › 본(계층·슬롯·첨부물) / 그리기 순서 / 제약 / 스킨 / 이벤트 / 애니메이션 / 이미지. 검색 거르기.
import type { Project } from '../../core/types';
import type { ImageEntry } from '../../platform/platform';
import { selKey, type Sel } from '../../state/selection';

export interface TreeNode {
  key: string;
  label: string;
  icon: string;
  sel: Sel | null;
  children: TreeNode[];
  /** 그리기 순서 줄(끌어서 순서 바꾸기) */
  drawOrderIndex?: number;
  visible?: boolean;
  folder?: boolean;
}

const node = (sel: Sel, label: string, icon: string, children: TreeNode[] = [], extra: Partial<TreeNode> = {}): TreeNode => ({
  key: selKey(sel),
  label,
  icon,
  sel,
  children,
  ...extra,
});
const folder = (key: string, label: string, icon: string, children: TreeNode[]): TreeNode => ({ key: `folder:${key}`, label, icon, sel: null, children, folder: true });

export function buildTree(p: Project, images: ImageEntry[]): TreeNode {
  const sk = p.skeleton;
  const activeSkin = p.editor.activeSkin;
  const skinsShown = sk.skins.filter((s) => s.name === 'default' || s.name === activeSkin);
  const slotNode = (slotName: string, visible: boolean, drawOrderIndex?: number): TreeNode => {
    const atts: TreeNode[] = [];
    for (const skin of skinsShown) {
      for (const name of Object.keys(skin.attachments[slotName] ?? {})) {
        const a = skin.attachments[slotName][name];
        atts.push(node({ kind: 'attachment', name, slot: slotName, skin: skin.name }, skin.name === 'default' ? name : `${name} (${skin.name})`, a.type === 'region' ? 'attachment' : 'image'));
      }
    }
    const n = node({ kind: 'slot', name: slotName }, slotName, 'slot', atts, { visible, drawOrderIndex });
    if (drawOrderIndex !== undefined) n.key = `draworder:${slotName}`;
    return n;
  };
  const boneNode = (name: string): TreeNode => {
    const b = sk.bones.find((x) => x.name === name)!;
    const kids = sk.bones.filter((x) => x.parent === name).map((x) => boneNode(x.name));
    const slots = sk.slots.filter((s) => s.bone === name).map((s) => slotNode(s.name, s.visible));
    return node({ kind: 'bone', name }, name, 'bone', [...kids, ...slots], { visible: b.visible });
  };
  const roots = sk.bones.filter((b) => !b.parent).map((b) => boneNode(b.name));
  const children: TreeNode[] = [
    ...roots,
    folder('draworder', '그리기 순서', 'folder', sk.slots.map((s, i) => slotNode(s.name, s.visible, i))),
    folder('constraints', '제약', 'folder', sk.constraints.map((c) => node({ kind: 'constraint', name: c.name }, c.name, 'constraint'))),
    folder('skins', '스킨', 'folder', sk.skins.map((s) => node({ kind: 'skin', name: s.name }, s.name + (s.name === activeSkin ? ' (사용 중)' : ''), 'skin'))),
    folder('events', '이벤트', 'folder', sk.events.map((e) => node({ kind: 'event', name: e.name }, e.name, 'event'))),
    folder('animations', '애니메이션', 'folder', sk.animations.map((a) => node({ kind: 'animation', name: a.name }, a.name + (a.name === p.editor.activeAnimation ? ' ▶' : ''), 'animation'))),
    folder('images', '이미지', 'folder', images.map((e) => node({ kind: 'image', name: e.name }, e.name, 'image'))),
  ];
  return { key: 'skeleton:', label: '뼈대', icon: 'skeleton', sel: { kind: 'skeleton', name: '' }, children };
}

/** 검색어가 들어간 노드와 그 조상만 남긴다 */
export function filterTree(n: TreeNode, q: string): TreeNode | null {
  const query = q.trim().toLowerCase();
  if (!query) return n;
  const kids = n.children.map((c) => filterTree(c, query)).filter((c): c is TreeNode => c !== null);
  if (kids.length || n.label.toLowerCase().includes(query)) return { ...n, children: kids };
  return null;
}
