// 도프시트: 타임라인 줄·키 마름모·선택·드래그 이동(key.move)·복사/붙여넣기(key.paste)·눈금 클릭 시점 이동.
import { useMemo, useRef, useState } from 'react';
import { copyKeys, sameKey } from '../../anim/keys';
import { animationDuration, timelineRows, type TimelineRow } from '../../anim/timeline';
import { setTime } from '../../commands/actions';
import { useServices } from '../../services';
import { useStore } from '../../state/store';
import type { KeyRef } from '../../state/ui';

const PX_PER_FRAME = 14;
const LABEL_W = 170;

export function Dopesheet() {
  const s = useServices();
  const anim = useStore(s.doc, (d) => d.project.skeleton.animations.find((a) => a.name === d.project.editor.activeAnimation) ?? null);
  const fps = useStore(s.doc, (d) => d.project.skeleton.fps || 30);
  const playing = useStore(s.ui, (u) => u.playing);
  const playTime = useStore(s.ui, (u) => u.playTime);
  const docTime = useStore(s.doc, (d) => d.project.editor.time);
  const selectedKeys = useStore(s.ui, (u) => u.selectedKeys);
  const graphRow = useStore(s.ui, (u) => u.graphRow);
  const clipboard = useRef<Record<string, unknown>[]>([]);
  const [dragFrames, setDragFrames] = useState(0);
  const drag = useRef<{ x: number; moved: boolean } | null>(null);
  const time = playing ? playTime : docTime;
  const rows = useMemo(() => (anim ? timelineRows(anim) : []), [anim]);
  const frames = Math.max(30, Math.ceil(animationDuration(anim) * fps) + 15);
  const width = frames * PX_PER_FRAME;

  if (!anim) return <div className="dope-empty">애니메이션을 고르거나 만들기 › 애니메이션으로 만드세요</div>;

  const refOf = (r: TimelineRow, t: number): KeyRef => ({ target: r.target, timeline: r.timeline, time: t });
  const isSel = (k: KeyRef) => selectedKeys.some((x) => sameKey(x, k));
  const timeAtX = (x: number) => Math.max(0, Math.round(x / PX_PER_FRAME)) / fps;

  const onKeyDown = (e: React.KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'c') {
      clipboard.current = copyKeys(anim, selectedKeys);
      s.ui.notify('info', `키 ${clipboard.current.length}개를 복사했습니다`);
      e.preventDefault();
      e.stopPropagation();
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'v') {
      if (clipboard.current.length) s.doc.run('key.paste', { keys: clipboard.current, time });
      e.preventDefault();
      e.stopPropagation();
    }
  };

  let lastGroup = '';
  return (
    <div className="dopesheet" tabIndex={0} onKeyDown={onKeyDown} data-testid="dopesheet">
      <div className="dope-scroll">
        <div className="dope-ruler-row" style={{ width: LABEL_W + width }}>
          <div className="dope-label dope-corner" style={{ width: LABEL_W }}>
            {anim.name}
          </div>
          <div
            className="dope-ruler"
            style={{ width }}
            data-testid="dope-ruler"
            onPointerDown={(e) => {
              const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
              setTime(s, timeAtX(e.clientX - rect.left));
            }}
          >
            {Array.from({ length: Math.ceil(frames / 5) + 1 }, (_, i) => (
              <span key={i} className="dope-tick" style={{ left: i * 5 * PX_PER_FRAME }}>
                {i * 5}
              </span>
            ))}
          </div>
        </div>
        {rows.map((r) => {
          const header = r.group !== lastGroup;
          lastGroup = r.group;
          return (
            <div key={r.id}>
              {header && (
                <div className="dope-row group" style={{ width: LABEL_W + width }}>
                  <div className="dope-label" style={{ width: LABEL_W }}>
                    {r.group}
                  </div>
                </div>
              )}
              <div className={'dope-row' + (graphRow === r.id ? ' active' : '')} style={{ width: LABEL_W + width }} data-row={r.id}>
                <div className="dope-label sub" style={{ width: LABEL_W }} onClick={() => s.ui.set({ graphRow: r.id })}>
                  {r.label}
                </div>
                <div className="dope-track" style={{ width }} onPointerDown={(e) => e.target === e.currentTarget && s.ui.set({ selectedKeys: [] })}>
                  {r.keys.map((k) => {
                    const ref = refOf(r, k.time);
                    const sel = isSel(ref);
                    return (
                      <span
                        key={k.time}
                        className={'dope-key' + (sel ? ' selected' : '')}
                        data-testid="dope-key"
                        title={`${Math.round(k.time * fps * 100) / 100}프레임`}
                        style={{ left: k.time * fps * PX_PER_FRAME + (sel ? dragFrames * PX_PER_FRAME : 0) }}
                        onPointerDown={(e) => {
                          e.stopPropagation();
                          (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
                          (e.currentTarget.closest('.dopesheet') as HTMLElement)?.focus();
                          if (e.shiftKey || e.ctrlKey) s.ui.set({ selectedKeys: sel ? selectedKeys.filter((x) => !sameKey(x, ref)) : [...selectedKeys, ref] });
                          else if (!sel) s.ui.set({ selectedKeys: [ref] });
                          s.ui.set({ graphRow: r.id });
                          drag.current = { x: e.clientX, moved: false };
                        }}
                        onPointerMove={(e) => {
                          const d = drag.current;
                          if (!d) return;
                          const f = Math.round((e.clientX - d.x) / PX_PER_FRAME);
                          if (f !== 0) d.moved = true;
                          setDragFrames(f);
                        }}
                        onPointerUp={() => {
                          const d = drag.current;
                          drag.current = null;
                          const f = dragFrames;
                          setDragFrames(0);
                          if (d?.moved && f !== 0) {
                            const keys = s.ui.get().selectedKeys;
                            const delta = f / fps;
                            if (s.doc.run('key.move', { keys, delta }).ok) s.ui.set({ selectedKeys: keys.map((x) => ({ ...x, time: Math.max(0, x.time + delta) })) });
                          }
                        }}
                      />
                    );
                  })}
                </div>
              </div>
            </div>
          );
        })}
        <div className="dope-playhead" style={{ left: LABEL_W + time * fps * PX_PER_FRAME }} />
      </div>
    </div>
  );
}
