// 그래프(곡선) 편집기: 고른 타임라인의 값 곡선, 베지어 손잡이 끌기(key.setCurve), 선형/계단/베지어 버튼.
import { useEffect, useMemo, useRef, useState } from 'react';
import { fitGraph, graphToScreen, moveHandle, screenToGraph, segmentPoints } from '../../anim/graph';
import { pickGraphRow, timelineRows } from '../../anim/timeline';
import type { Key } from '../../core/types';
import { useServices } from '../../services';
import { formatNumber } from '../fields/numberInput';
import { useStore } from '../../state/store';

const CHANNEL_COLORS = ['#e8913a', '#5fb3f0', '#7fd36a', '#d36ab8', '#e0d050', '#aaaaaa', '#ff7070'];

export function GraphEditor() {
  const s = useServices();
  const anim = useStore(s.doc, (d) => d.project.skeleton.animations.find((a) => a.name === d.project.editor.activeAnimation) ?? null);
  const graphRow = useStore(s.ui, (u) => u.graphRow);
  const fps = useStore(s.doc, (d) => d.project.skeleton.fps || 30);
  const selection = useStore(s.ui, (u) => u.selection);
  const wrap = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 600, h: 200 });
  const [channel, setChannel] = useState(0);
  const [dragCurve, setDragCurve] = useState<{ index: number; curve: number[] } | null>(null);
  const drag = useRef<{ index: number; which: 1 | 2 } | null>(null);

  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setSize({ w: el.clientWidth, h: el.clientHeight }));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const row = useMemo(() => (anim ? pickGraphRow(timelineRows(anim), graphRow, selection) : null), [anim, graphRow, selection]);
  const keys: Key[] = (row?.keys.map((k) => k.key).filter(Boolean) as Key[]) ?? [];
  const channels = Math.max(1, ...keys.map((k) => k.v.length));
  const ch = Math.min(channel, channels - 1);
  const view = fitGraph(keys, ch, size.w, size.h, 20);

  const curveAt = (i: number): Key['curve'] => (dragCurve?.index === i ? dragCurve.curve : keys[i].curve);

  const toolbar = (
    <div className="graph-toolbar">
      <button onClick={() => s.run('key.setCurve', { curve: 'linear' })}>선형</button>
      <button onClick={() => s.run('key.setCurve', { curve: 'stepped' })}>계단</button>
      <button onClick={() => s.run('key.setCurve', { curve: 'bezier' })}>베지어</button>
      {channels > 1 && (
        <select aria-label="채널" value={ch} onChange={(e) => setChannel(Number(e.target.value))}>
          {Array.from({ length: channels }, (_, i) => (
            <option key={i} value={i}>
              채널 {i + 1}
            </option>
          ))}
        </select>
      )}
      <span className="graph-title" data-testid="graph-title">{row ? `${row.group} › ${row.label}` : anim ? '이 애니메이션에는 아직 키가 없습니다 (K로 키 지정)' : '애니메이션을 고르세요'}</span>
    </div>
  );

  const pointer = (e: React.PointerEvent) => {
    const rect = wrap.current!.getBoundingClientRect();
    return screenToGraph(view, e.clientX - rect.left, e.clientY - rect.top);
  };

  return (
    <div className="graph">
      {toolbar}
      <div className="graph-area" ref={wrap}>
        <svg
          width={size.w}
          height={size.h}
          onPointerMove={(e) => {
            const d = drag.current;
            if (!d) return;
            const p = pointer(e);
            const k = keys[d.index];
            const next = keys[d.index + 1];
            const cur = curveAt(d.index);
            if (!Array.isArray(cur) || !next) return;
            setDragCurve({ index: d.index, curve: moveHandle(cur, ch, d.which, p.t, p.v, k.time, next.time) });
          }}
          onPointerUp={() => {
            const d = drag.current;
            drag.current = null;
            if (d && dragCurve && row) s.doc.run('key.setCurve', { keys: [{ target: row.target, timeline: row.timeline, time: keys[d.index].time }], curve: dragCurve.curve });
            setDragCurve(null);
          }}
        >
          {Array.from({ length: 5 }, (_, i) => {
            const v = view.v0 + ((view.v1 - view.v0) * i) / 4;
            const t = view.t0 + ((view.t1 - view.t0) * i) / 4;
            const ph = graphToScreen(view, view.t0, v);
            const pv = graphToScreen(view, t, view.v0);
            return (
              <g key={`grid${i}`} className="graph-grid">
                <line x1={0} y1={ph.y} x2={size.w} y2={ph.y} />
                <line x1={pv.x} y1={0} x2={pv.x} y2={size.h} />
                <text x={4} y={ph.y - 2}>
                  {formatNumber(v, 2)}
                </text>
                <text x={pv.x + 2} y={size.h - 4}>
                  {formatNumber(t * fps, 1)}f
                </text>
              </g>
            );
          })}
          {row &&
            keys.map((k, i) => {
              const next = keys[i + 1];
              if (!next) return null;
              const curve = curveAt(i);
              const pts = segmentPoints(k, next, ch, curve).map((p) => graphToScreen(view, p.t, p.v));
              const handles = Array.isArray(curve) && curve.length >= ch * 4 + 4;
              const a = graphToScreen(view, k.time, k.v[ch] ?? 0);
              const b = graphToScreen(view, next.time, next.v[ch] ?? 0);
              const h1 = handles ? graphToScreen(view, curve[ch * 4], curve[ch * 4 + 1]) : null;
              const h2 = handles ? graphToScreen(view, curve[ch * 4 + 2], curve[ch * 4 + 3]) : null;
              return (
                <g key={i}>
                  <polyline points={pts.map((p) => `${p.x},${p.y}`).join(' ')} fill="none" stroke={CHANNEL_COLORS[ch % CHANNEL_COLORS.length]} strokeWidth={1.5} />
                  {h1 && h2 && (
                    <>
                      <line x1={a.x} y1={a.y} x2={h1.x} y2={h1.y} className="graph-handle-line" />
                      <line x1={b.x} y1={b.y} x2={h2.x} y2={h2.y} className="graph-handle-line" />
                      {([1, 2] as const).map((w) => {
                        const h = w === 1 ? h1 : h2;
                        return (
                          <circle
                            key={w}
                            cx={h.x}
                            cy={h.y}
                            r={5}
                            className="graph-handle"
                            data-testid="graph-handle"
                            onPointerDown={(e) => {
                              (e.currentTarget.ownerSVGElement as SVGSVGElement).setPointerCapture(e.pointerId);
                              drag.current = { index: i, which: w };
                            }}
                          />
                        );
                      })}
                    </>
                  )}
                </g>
              );
            })}
          {row &&
            keys.map((k, i) => {
              const p = graphToScreen(view, k.time, k.v[ch] ?? 0);
              return <rect key={`k${i}`} x={p.x - 4} y={p.y - 4} width={8} height={8} transform={`rotate(45 ${p.x} ${p.y})`} className="graph-key" data-testid="graph-key" />;
            })}
        </svg>
      </div>
    </div>
  );
}
