// 뷰포트: WebGL2 미리보기, 확대(휠)·이동(가운데 버튼/Space+드래그), 클릭 선택, 드래그 도구.
import { useEffect, useRef, useState } from 'react';
import { animationDuration } from '../../anim/timeline';
import type { PoseOutput } from '../../core/types';
import { screenToWorld, zoomAt } from '../../render/camera';
import { hitBone, hitSlot } from '../../render/hitTest';
import { onionTimes } from '../../render/onion';
import { Renderer, type FrameInput } from '../../render/renderer';
import type { Rgba } from '../../render/shapes';
import { useServices, type Services } from '../../services';
import { namesOf, sameSel, type Sel } from '../../state/selection';
import { useStore } from '../../state/store';
import { axisFrame } from '../../tools/axis';
import { IDENTITY } from '../../tools/affine';
import { createDragTool, type DragTool } from '../../tools/dragTools';
import { hitVertex } from '../../tools/meshEdit';
import { addVertexAt, meshEditTarget, VertexDrag } from '../../tools/meshEditSession';

const BEFORE_TINT: Rgba = [1, 0.4, 0.4, 0.28];
const AFTER_TINT: Rgba = [0.4, 0.7, 1, 0.28];

function onionPoses(s: Services, time: number): FrameInput['onion'] {
  const p = s.doc.get().project;
  const ed = p.editor;
  if (!s.ui.get().onion || ed.mode !== 'animate' || !ed.activeAnimation) return [];
  const anim = p.skeleton.animations.find((a) => a.name === ed.activeAnimation);
  const times = onionTimes(time, { before: ed.onion.before, after: ed.onion.after, step: ed.onion.step, fps: p.skeleton.fps, duration: animationDuration(anim), loop: s.ui.get().loop });
  const out: FrameInput['onion'] = [];
  times.before.forEach((t, i) => out.push({ pose: s.pose.pose(s.pose.request(t)), tint: [BEFORE_TINT[0], BEFORE_TINT[1], BEFORE_TINT[2], BEFORE_TINT[3] / (i + 1)] }));
  times.after.forEach((t, i) => out.push({ pose: s.pose.pose(s.pose.request(t)), tint: [AFTER_TINT[0], AFTER_TINT[1], AFTER_TINT[2], AFTER_TINT[3] / (i + 1)] }));
  return out;
}

/** 변환 축 표시, 메시 정점 편집 점 */
function extraGizmo(s: Services, pose: PoseOutput): FrameInput['gizmo'] {
  const u = s.ui.get();
  const g: FrameInput['gizmo'] = {};
  if (u.meshEdit) {
    const t = meshEditTarget(s);
    if (t) g.vertices = { points: t.world, selected: u.selectedVertices, hull: t.mesh.hull };
    return g;
  }
  if (!['rotate', 'translate', 'scale', 'shear'].includes(u.tool)) return g;
  const bones = s.doc.get().project.skeleton.bones;
  const name = namesOf(u.selection, 'bone').at(-1);
  const i = bones.findIndex((b) => b.name === name);
  const m = pose.bones[i];
  if (!m) return g;
  const pi = bones.findIndex((b) => b.name === bones[i].parent);
  const f = axisFrame(u.axis, m, pose.bones[pi] ?? IDENTITY);
  g.axes = { x: m.x, y: m.y, ...f };
  return g;
}

export function Viewport() {
  const s = useServices();
  const wrap = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const renderer = useRef<Renderer | null>(null);
  const [noGl, setNoGl] = useState(false);
  const mode = useStore(s.doc, (d) => d.project.editor.mode);
  const pick = useStore(s.ui, (u) => u.pickParent);
  const tool = useStore(s.ui, (u) => u.tool);
  const meshEdit = useStore(s.ui, (u) => u.meshEdit);
  const frame = useRef(0);
  const livePose = useRef<PoseOutput | null>(null);
  const drag = useRef<DragTool | null>(null);
  const pan = useRef<{ x: number; y: number; vx: number; vy: number } | null>(null);
  const space = useRef(false);
  const hover = useRef<string | null>(null);

  // 그리기
  useEffect(() => {
    const c = canvas.current!;
    const r = Renderer.create(c, () => request());
    renderer.current = r;
    if (!r) {
      setNoGl(true);
      s.ui.notify('warn', 'WebGL2를 쓸 수 없어 미리보기를 그리지 못합니다');
    }
    const draw = () => {
      frame.current = 0;
      const rr = renderer.current;
      if (!rr) return;
      const u = s.ui.get();
      const p = s.doc.get().project;
      const pose = (u.playing && livePose.current) || s.pose.pose();
      const sel = u.selection;
      const imgs = s.images;
      rr.draw({
        view: u.view,
        size: u.viewSize,
        dpr: window.devicePixelRatio || 1,
        pose,
        bones: p.skeleton.bones,
        onion: onionPoses(s, s.pose.currentTime()),
        showBones: u.showBones,
        showImages: u.showImages,
        selectedBones: new Set(namesOf(sel, 'bone')),
        selectedSlots: new Set([...namesOf(sel, 'slot'), ...sel.filter((x) => x.kind === 'attachment').map((x) => x.slot ?? '')]),
        hoverBone: hover.current,
        imageUrl: (name) => imgs.byName(name)?.url ?? null,
        gizmo: { ...(drag.current?.gizmo() ?? {}), ...extraGizmo(s, pose) },
      });
    };
    const request = () => {
      if (!frame.current) frame.current = requestAnimationFrame(draw);
    };
    let lastImages = s.images.get().version;
    const unsubs = [
      s.doc.subscribe(request),
      s.ui.subscribe(request),
      s.images.subscribe(() => {
        const v = s.images.get().version;
        if (v !== lastImages) {
          lastImages = v;
          renderer.current?.textures.clear();
        }
        request();
      }),
    ];
    s.player.onFrame = (dt, lastTime) => {
      const req = s.pose.request(s.ui.get().playTime);
      livePose.current = s.pose.compute({ ...req, lastTime, physics: dt });
      request();
    };
    const ro = new ResizeObserver(() => {
      const el = wrap.current;
      if (!el) return;
      s.ui.set({ viewSize: { w: el.clientWidth, h: el.clientHeight } });
      request();
    });
    ro.observe(wrap.current!);
    request();
    return () => {
      unsubs.forEach((u) => u());
      ro.disconnect();
      s.player.onFrame = null;
      cancelAnimationFrame(frame.current);
      frame.current = 0;
    };
  }, [s]);

  // Space 누르는 동안 화면 이동
  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.code === 'Space' && !(e.target as HTMLElement).closest?.('input,textarea,select')) space.current = true;
    };
    const up = (e: KeyboardEvent) => {
      if (e.code === 'Space') space.current = false;
    };
    window.addEventListener('keydown', down);
    window.addEventListener('keyup', up);
    return () => {
      window.removeEventListener('keydown', down);
      window.removeEventListener('keyup', up);
    };
  }, []);

  const worldAt = (e: { clientX: number; clientY: number }) => {
    // 테두리(애니메이트 모드 주황) 안쪽 기준 좌표
    const el = wrap.current!;
    const rect = el.getBoundingClientRect();
    const u = s.ui.get();
    const sx = e.clientX - rect.left - el.clientLeft;
    const sy = e.clientY - rect.top - el.clientTop;
    return { sx, sy, ...screenToWorld(u.view, u.viewSize, sx, sy) };
  };

  const onPointerDown = (e: React.PointerEvent) => {
    (document.activeElement as HTMLElement | null)?.blur?.();
    wrap.current?.focus();
    const u = s.ui.get();
    if (e.button === 1 || (e.button === 0 && space.current)) {
      e.preventDefault();
      pan.current = { x: e.clientX, y: e.clientY, vx: u.view.x, vy: u.view.y };
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      return;
    }
    if (e.button !== 0) return;
    const w = worldAt(e);
    const p = s.doc.get().project;
    const pose = s.pose.pose();
    const tol = 8 / u.view.zoom;

    // 메시 정점 편집: 정점 클릭·끌기, 빈 곳 클릭 = 정점 추가
    if (u.meshEdit) {
      const t = meshEditTarget(s);
      if (!t) {
        s.ui.set({ meshEdit: false, selectedVertices: [] });
        s.ui.notify('warn', '정점 편집: 보이는 메시 첨부물을 고르세요');
        return;
      }
      const vi = hitVertex(t.world, w.x, w.y, tol);
      if (vi < 0) {
        addVertexAt(s, t, w);
        return;
      }
      let sel = u.selectedVertices;
      if (e.shiftKey || e.ctrlKey) sel = sel.includes(vi) ? sel.filter((x) => x !== vi) : [...sel, vi];
      else if (!sel.includes(vi)) sel = [vi];
      s.ui.set({ selectedVertices: sel });
      if (!sel.includes(vi)) return;
      drag.current = new VertexDrag(s, t, sel);
      drag.current.begin(w, e);
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      return;
    }
    const bone = u.showBones ? hitBone(pose, p.skeleton.bones, w.x, w.y, tol) : null;

    if (u.pickParent) {
      if (bone) {
        s.doc.group('부모 변경', () => {
          for (const name of namesOf(u.selection, 'bone')) if (name !== bone) s.doc.run('bone.setParent', { name, parent: bone });
        });
      }
      s.ui.set({ pickParent: false });
      return;
    }

    let target: Sel | null = bone ? { kind: 'bone', name: bone } : null;
    if (!target && u.tool !== 'create' && u.tool !== 'weights') {
      const hs = u.showImages ? hitSlot(pose, w.x, w.y) : null;
      if (hs) target = { kind: 'slot', name: hs.slot };
    }
    if (target) {
      const already = u.selection.some((x) => sameSel(x, target!));
      if (e.ctrlKey || e.shiftKey) {
        s.ui.select(already ? u.selection.filter((x) => !sameSel(x, target!)) : [...u.selection, target]);
      } else if (!already) {
        // 가중치 도구에서는 첨부물 선택을 지킨다
        const keep = u.tool === 'weights' ? u.selection.filter((x) => x.kind === 'attachment') : [];
        s.ui.select([...keep, target]);
      }
    } else if (u.tool !== 'create' && u.tool !== 'weights' && !e.ctrlKey) {
      s.ui.select([]);
    }
    const needsBone = u.tool !== 'create' && u.tool !== 'weights';
    if (needsBone && target?.kind !== 'bone') return;
    if (u.tool === 'create' && bone) return;
    drag.current = createDragTool(s, u.tool);
    drag.current.begin(w, e);
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  };

  const onPointerMove = (e: React.PointerEvent) => {
    const w = worldAt(e);
    s.ui.set({ cursor: { x: w.x, y: w.y } });
    if (pan.current) {
      const z = s.ui.get().view.zoom;
      s.ui.set({ view: { x: pan.current.vx - (e.clientX - pan.current.x) / z, y: pan.current.vy + (e.clientY - pan.current.y) / z, zoom: z } });
      return;
    }
    if (drag.current) {
      drag.current.move(w, e);
      return;
    }
    const u = s.ui.get();
    const h = u.showBones ? hitBone(s.pose.pose(), s.doc.get().project.skeleton.bones, w.x, w.y, 8 / u.view.zoom) : null;
    if (h !== hover.current) {
      hover.current = h;
      s.ui.set({ cursor: { x: w.x, y: w.y } });
    }
  };

  const onPointerUp = () => {
    pan.current = null;
    const d = drag.current;
    drag.current = null;
    d?.end();
  };

  return (
    <div
      className={'viewport' + (mode === 'animate' ? ' animate' : '') + (pick ? ' picking' : '') + (meshEdit ? ' mesh-edit' : '') + ` tool-${tool}`}
      ref={wrap}
      tabIndex={-1}
      data-testid="viewport"
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerLeave={() => s.ui.set({ cursor: null })}
      onWheel={(e) => {
        const w = worldAt(e);
        const u = s.ui.get();
        s.ui.set({ view: zoomAt(u.view, u.viewSize, w.sx, w.sy, u.view.zoom * (e.deltaY < 0 ? 1.15 : 1 / 1.15)) });
      }}
      onContextMenu={(e) => {
        e.preventDefault();
        s.ui.swapTool();
      }}
    >
      <canvas ref={canvas} />
      {noGl && <div className="viewport-msg">WebGL2를 쓸 수 없습니다</div>}
      {meshEdit && (
        <div className="viewport-banner" data-testid="mesh-edit-banner">
          정점 편집: 점 끌기 = 옮기기 · 빈 곳 클릭 = 정점 추가 · Delete = 고른 정점 삭제 · Esc = 끝내기
        </div>
      )}
    </div>
  );
}
