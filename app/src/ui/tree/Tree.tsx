// 트리 패널(오른쪽 위): 검색칸, 계층, 눈 아이콘, 키 표시, 끌어 놓기(그리기 순서·부모), 이름 바꾸기, 오른쪽 클릭 메뉴.
import { useMemo, useState } from 'react';
import {
  attachmentTypeLabel,
  constraintTypeLabel,
  createAttachment,
  createBone,
  createConstraint,
  createSimple,
  createSlot,
  deleteSelection,
  renameItem,
} from '../../commands/actions';
import { hasKeyAt } from '../../anim/timeline';
import type { AttachmentType, ConstraintType } from '../../core/types';
import { useServices, type Services } from '../../services';
import { sameSel, selKey, type Sel } from '../../state/selection';
import { useStore } from '../../state/store';
import { TextField } from '../fields/TextField';
import { Icon } from '../icons';
import { ContextMenu, type ContextMenuState } from '../panels/ContextMenu';
import type { ListItem } from '../panels/MenuList';
import { buildTree, filterTree, type TreeNode } from './buildTree';

const ATT_TYPES: AttachmentType[] = ['region', 'mesh', 'linkedmesh', 'boundingbox', 'point', 'clipping', 'path'];
const CON_TYPES: ConstraintType[] = ['ik', 'transform', 'path', 'physics', 'slider'];

function contextItems(s: Services, n: TreeNode): ListItem[] {
  const items: ListItem[] = [];
  const act = (label: string, run: () => void): ListItem => ({ type: 'action', label, run });
  const kind = n.sel?.kind;
  if (kind === 'bone' || kind === 'skeleton') {
    items.push(act('본 만들기', () => createBone(s)), act('슬롯 만들기', () => createSlot(s)));
  }
  if (kind === 'slot' || kind === 'attachment') {
    for (const t of ATT_TYPES) items.push(act(`첨부물 만들기: ${attachmentTypeLabel(t)}`, () => void createAttachment(s, t)));
  }
  if (kind === 'image') {
    items.push(act('선택한 슬롯에 이미지 붙이기', () => void createAttachment(s, 'region', n.sel!.name)));
  }
  if (n.key === 'folder:constraints' || kind === 'bone') for (const t of CON_TYPES) items.push(act(`제약 만들기: ${constraintTypeLabel(t)}`, () => createConstraint(s, t)));
  if (n.key === 'folder:skins') items.push(act('스킨 만들기', () => createSimple(s, 'skin')));
  if (n.key === 'folder:events') items.push(act('이벤트 만들기', () => createSimple(s, 'event')));
  if (n.key === 'folder:animations') items.push(act('애니메이션 만들기', () => createSimple(s, 'animation')));
  if (kind === 'skin') items.push(act('이 스킨 사용', () => s.doc.run('skin.setActive', { name: n.sel!.name })));
  if (kind === 'animation') items.push(act('이 애니메이션 고르기', () => s.doc.run('animation.setActive', { name: n.sel!.name })));
  if (n.sel && kind !== 'skeleton' && kind !== 'image') {
    if (items.length) items.push({ type: 'sep' });
    items.push({ type: 'command', id: 'item.rename' }, { type: 'command', id: 'edit.delete', label: '삭제' });
  }
  if (kind === 'animation') items.push({ type: 'command', id: 'edit.duplicate' });
  return items;
}

interface RowProps {
  n: TreeNode;
  depth: number;
  expanded: Set<string>;
  toggle(key: string): void;
  onMenu(m: ContextMenuState): void;
}

function Row({ n, depth, expanded, toggle, onMenu }: RowProps) {
  const s = useServices();
  const selection = useStore(s.ui, (u) => u.selection);
  const renaming = useStore(s.ui, (u) => u.renaming);
  const animate = useStore(s.doc, (d) => d.project.editor.mode === 'animate');
  const anim = useStore(s.doc, (d) => d.project.skeleton.animations.find((a) => a.name === d.project.editor.activeAnimation) ?? null);
  const time = useStore(s.doc, (d) => d.project.editor.time);
  const [dropHint, setDropHint] = useState<'' | 'above' | 'into'>('');
  const sel = n.sel;
  const isSelected = !!sel && selection.some((x) => sameSel(x, sel));
  const isOpen = expanded.has(n.key);
  const isRenaming = !!sel && renaming === selKey(sel) && n.drawOrderIndex === undefined;
  const keyed = animate && sel && (sel.kind === 'bone' || sel.kind === 'slot' || sel.kind === 'constraint') && hasKeyAt(anim, sel.kind, sel.name, time);

  const click = (e: React.MouseEvent) => {
    if (!sel) {
      toggle(n.key);
      return;
    }
    if (s.ui.get().pickParent && sel.kind === 'bone') {
      s.doc.group('부모 변경', () => {
        for (const x of s.ui.get().selection) if (x.kind === 'bone' && x.name !== sel.name) s.doc.run('bone.setParent', { name: x.name, parent: sel.name });
      });
      s.ui.set({ pickParent: false });
      return;
    }
    const cur = s.ui.get().selection;
    if (e.ctrlKey || e.shiftKey) s.ui.select(isSelected ? cur.filter((x) => !sameSel(x, sel)) : [...cur, sel]);
    else s.ui.select([sel]);
  };

  return (
    <>
      <div
        role="treeitem"
        aria-label={sel?.name || n.label}
        aria-selected={isSelected}
        aria-expanded={n.children.length ? isOpen : undefined}
        data-kind={sel?.kind ?? 'folder'}
        data-name={sel?.name ?? n.label}
        className={'tree-row' + (isSelected ? ' selected' : '') + (dropHint ? ` drop-${dropHint}` : '')}
        style={{ paddingLeft: 4 + depth * 14 }}
        draggable={!!sel && (sel.kind === 'bone' || n.drawOrderIndex !== undefined)}
        onDragStart={(e) => {
          e.dataTransfer.setData('text/kgbone-node', JSON.stringify({ sel, drawOrderIndex: n.drawOrderIndex }));
          e.dataTransfer.effectAllowed = 'move';
        }}
        onDragOver={(e) => {
          if (!sel) return;
          if (n.drawOrderIndex !== undefined || sel.kind === 'bone') {
            e.preventDefault();
            setDropHint(n.drawOrderIndex !== undefined ? 'above' : 'into');
          }
        }}
        onDragLeave={() => setDropHint('')}
        onDrop={(e) => {
          setDropHint('');
          const raw = e.dataTransfer.getData('text/kgbone-node');
          if (!raw || !sel) return;
          e.preventDefault();
          const src = JSON.parse(raw) as { sel: Sel; drawOrderIndex?: number };
          if (n.drawOrderIndex !== undefined && src.drawOrderIndex !== undefined) {
            let index = n.drawOrderIndex;
            if (src.drawOrderIndex < index) index -= 1;
            if (src.drawOrderIndex !== index) s.doc.run('slot.reorder', { name: src.sel.name, index });
          } else if (sel.kind === 'bone' && src.sel.kind === 'bone' && src.sel.name !== sel.name) {
            s.doc.run('bone.setParent', { name: src.sel.name, parent: sel.name });
          }
        }}
        onClick={click}
        onDoubleClick={() => {
          if (sel && sel.kind !== 'skeleton' && sel.kind !== 'image') {
            s.ui.select([sel]);
            s.ui.set({ renaming: selKey(sel) });
          } else toggle(n.key);
        }}
        onContextMenu={(e) => {
          e.preventDefault();
          if (sel && !isSelected) s.ui.select([sel]);
          onMenu({ x: e.clientX, y: e.clientY, items: contextItems(s, n) });
        }}
      >
        <span
          className="tree-twisty"
          onClick={(e) => {
            e.stopPropagation();
            toggle(n.key);
          }}
        >
          {n.children.length ? <Icon name={isOpen ? 'chevronDown' : 'chevron'} size={12} /> : null}
        </span>
        <Icon name={n.icon} size={14} />
        {isRenaming && sel ? (
          <TextField
            className="tree-rename"
            aria-label="새 이름"
            value={sel.name}
            autoSelect
            onClick={(e) => e.stopPropagation()}
            onCommit={(v) => {
              s.ui.set({ renaming: null });
              renameItem(s, sel, v);
            }}
            onCancel={() => s.ui.set({ renaming: null })}
            onBlur={() => s.ui.get().renaming === selKey(sel) && s.ui.set({ renaming: null })}
          />
        ) : (
          <span className="tree-label">{n.label}</span>
        )}
        {keyed && <span className="tree-key" title="현재 시점에 키가 있음"><Icon name="key" size={10} /></span>}
        {n.visible !== undefined && sel && (sel.kind === 'bone' || sel.kind === 'slot') && (
          <button
            className="tree-eye"
            title={n.visible ? '숨기기' : '보이기'}
            onClick={(e) => {
              e.stopPropagation();
              s.doc.run(sel.kind === 'bone' ? 'bone.setProps' : 'slot.setProps', { name: sel.name, visible: !n.visible });
            }}
          >
            <Icon name={n.visible ? 'eye' : 'eyeOff'} size={14} />
          </button>
        )}
      </div>
      {isOpen && n.children.map((c) => <Row key={c.key} n={c} depth={depth + 1} expanded={expanded} toggle={toggle} onMenu={onMenu} />)}
    </>
  );
}

export function Tree() {
  const s = useServices();
  const project = useStore(s.doc, (d) => d.project);
  const images = useStore(s.images, (i) => i.entries);
  const filter = useStore(s.ui, (u) => u.treeFilter);
  const selection = useStore(s.ui, (u) => u.selection);
  const [manual, setManual] = useState<Record<string, boolean>>({ 'skeleton:': true, 'folder:draworder': false, 'folder:images': false });
  const [menu, setMenu] = useState<ContextMenuState | null>(null);
  const root = useMemo(() => buildTree(project, images), [project, images]);
  const shown = useMemo(() => filterTree(root, filter), [root, filter]);

  // 펼침: 직접 접고 편 것 우선, 기본은 뼈대·본·슬롯·폴더 펼침, 검색 중이면 모두 펼침
  const expanded = useMemo(() => {
    const set = new Set<string>();
    const selKeys = new Set(selection.map(selKey));
    const walk = (n: TreeNode, ancestors: string[]) => {
      const m = manual[n.key];
      const def = n.key === 'skeleton:' || n.sel?.kind === 'bone' || (n.sel?.kind === 'slot' && n.drawOrderIndex === undefined) || (n.folder && n.key !== 'folder:images' && n.key !== 'folder:draworder');
      if (filter.trim() || m === true || (m === undefined && def)) set.add(n.key);
      if (n.sel && selKeys.has(selKey(n.sel)) && n.drawOrderIndex === undefined) ancestors.forEach((a) => set.add(a));
      n.children.forEach((c) => walk(c, [...ancestors, n.key]));
    };
    walk(root, []);
    return set;
  }, [root, manual, filter, selection]);

  const toggle = (key: string) => setManual((m) => ({ ...m, [key]: !expanded.has(key) }));

  return (
    <div className="tree-panel panel">
      <div className="panel-title">트리</div>
      <div className="tree-search">
        <TextField
          aria-label="트리 검색"
          placeholder="검색 (Ctrl+F)"
          value={filter}
          data-testid="tree-search"
          onLive={(v) => s.ui.set({ treeFilter: v })}
          onCommit={(v) => s.ui.set({ treeFilter: v })}
          onCancel={() => s.ui.set({ treeFilter: '' })}
        />
      </div>
      <div
        className="tree-body"
        role="tree"
        aria-label="뼈대 트리"
        onKeyDown={(e) => {
          if (e.key === 'Delete' && !(e.target as HTMLElement).closest('input')) {
            e.stopPropagation();
            deleteSelection(s);
          }
        }}
      >
        {shown ? <Row n={shown} depth={0} expanded={expanded} toggle={toggle} onMenu={setMenu} /> : <div className="tree-empty">찾는 항목이 없습니다</div>}
      </div>
      {menu && <ContextMenu menu={menu} onClose={() => setMenu(null)} />}
    </div>
  );
}
