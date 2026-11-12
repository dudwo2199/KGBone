// 오른쪽 클릭 메뉴(화면 좌표에 띄움).
import { useEffect, useRef } from 'react';
import { MenuList, type ListItem } from './MenuList';

export interface ContextMenuState {
  x: number;
  y: number;
  items: ListItem[];
}

export function ContextMenu({ menu, onClose }: { menu: ContextMenuState; onClose(): void }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const close = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) onClose();
    };
    const esc = (e: KeyboardEvent) => e.key === 'Escape' && onClose();
    window.addEventListener('mousedown', close);
    window.addEventListener('keydown', esc, true);
    return () => {
      window.removeEventListener('mousedown', close);
      window.removeEventListener('keydown', esc, true);
    };
  }, [onClose]);
  return (
    <div className="context-menu" ref={ref} style={{ left: menu.x, top: menu.y }}>
      <MenuList items={menu.items} onDone={onClose} />
    </div>
  );
}
