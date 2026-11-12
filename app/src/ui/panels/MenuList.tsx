// 메뉴 목록(메뉴 막대 펼침·오른쪽 클릭 메뉴 공용). 항목은 명령 id를 부른다.
import { useState } from 'react';
import { menuItemLabel, menuItemShortcut, type MenuItem } from '../../commands/menus';
import { useServices } from '../../services';
import { useStore } from '../../state/store';

export interface ActionItem {
  type: 'action';
  label: string;
  run(): void;
  disabled?: boolean;
}
export type ListItem = MenuItem | ActionItem;

export function MenuList({ items, onDone }: { items: ListItem[]; onDone(): void }) {
  const s = useServices();
  const canUndo = useStore(s.doc, (d) => d.docState.canUndo);
  const canRedo = useStore(s.doc, (d) => d.docState.canRedo);
  const [openSub, setOpenSub] = useState<number | null>(null);
  return (
    <div className="menu-list" role="menu">
      {items.map((it, i) => {
        if (it.type === 'sep') return <div key={i} className="menu-sep" />;
        if (it.type === 'sub')
          return (
            <div key={i} className="menu-item sub" role="menuitem" aria-haspopup="true" onMouseEnter={() => setOpenSub(i)} onClick={() => setOpenSub(i)}>
              <span>{it.label}</span>
              <span className="menu-arrow">▸</span>
              {openSub === i && (
                <div className="submenu">
                  <MenuList items={it.items} onDone={onDone} />
                </div>
              )}
            </div>
          );
        if (it.type === 'action')
          return (
            <div
              key={i}
              className={'menu-item' + (it.disabled ? ' disabled' : '')}
              role="menuitem"
              onMouseEnter={() => setOpenSub(null)}
              onClick={() => {
                if (it.disabled) return;
                onDone();
                it.run();
              }}
            >
              <span>{it.label}</span>
            </div>
          );
        const disabled = (it.id === 'edit.undo' && !canUndo) || (it.id === 'edit.redo' && !canRedo);
        return (
          <div
            key={i}
            className={'menu-item' + (disabled ? ' disabled' : '')}
            role="menuitem"
            data-command={it.id}
            onMouseEnter={() => setOpenSub(null)}
            onClick={() => {
              if (disabled) return;
              onDone();
              s.run(it.id, it.args ?? {});
            }}
          >
            <span>{menuItemLabel(it)}</span>
            <span className="menu-shortcut">{menuItemShortcut(it)}</span>
          </div>
        );
      })}
    </div>
  );
}
