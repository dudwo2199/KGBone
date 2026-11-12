// 메뉴 막대(UI_SPEC 2장) + 창 제목 표시.
import { useEffect, useRef, useState } from 'react';
import { MENUS } from '../../commands/menus';
import { useServices } from '../../services';
import { useStore } from '../../state/store';
import { MenuList } from './MenuList';

export function MenuBar() {
  const s = useServices();
  const [open, setOpen] = useState<number | null>(null);
  const ref = useRef<HTMLDivElement>(null);
  const dirty = useStore(s.doc, (d) => d.docState.dirty);
  useStore(s.doc, (d) => d.filePath ?? d.untitledName);
  const title = `${s.doc.displayName()}${dirty ? '*' : ''}`;

  useEffect(() => {
    if (open === null) return;
    const close = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(null);
    };
    window.addEventListener('mousedown', close);
    return () => window.removeEventListener('mousedown', close);
  }, [open]);

  return (
    <div className="menubar" ref={ref}>
      <div className="app-mark" title="KGBone">
        <svg width="18" height="18" viewBox="0 0 16 16" aria-hidden>
          <path d="M3 13L6 6l7-3-3 7z" fill="#e8913a" />
          <circle cx="3" cy="13" r="1.6" fill="#ddd" />
        </svg>
        <span>KGBone</span>
      </div>
      {MENUS.map((m, i) => (
        <div key={m.label} className={'menubar-entry' + (open === i ? ' open' : '')}>
          <button
            className="menubar-button"
            onMouseDown={(e) => {
              e.preventDefault();
              setOpen(open === i ? null : i);
            }}
            onMouseEnter={() => open !== null && setOpen(i)}
          >
            {m.label}
          </button>
          {open === i && (
            <div className="menu-dropdown">
              <MenuList items={m.items} onDone={() => setOpen(null)} />
            </div>
          )}
        </div>
      ))}
      <div className="menubar-title" data-testid="doc-title">
        {title}
      </div>
    </div>
  );
}
