// 명령 처리기 등록: scope=ui/platform은 화면 처리기, scope=core 중 선택 상태로 인자를 채워야 하는 것은 덮어쓴다.
import { defaultBezier } from '../anim/graph';
import { keyPair } from '../anim/keys';
import { snapToFrame } from '../anim/playback';
import { animationDuration, keyTimes, nextKeyTime, prevKeyTime } from '../anim/timeline';
import type { AttachmentType, ConstraintType } from '../core/types';
import { fitBounds, zoomAt } from '../render/camera';
import { poseBounds } from '../render/hitTest';
import type { Services } from '../services';
import { namesOf, selKey } from '../state/selection';
import type { ToolId } from '../state/ui';
import {
  createAttachment,
  createBone,
  createConstraint,
  createSimple,
  createSlot,
  deleteSelection,
  primary,
  setTime,
} from './actions';
import { CommandRegistry, type Args } from './registry';

function activeAnimation(s: Services) {
  const p = s.doc.get().project;
  return p.skeleton.animations.find((a) => a.name === p.editor.activeAnimation) ?? null;
}

function fps(s: Services): number {
  return s.doc.get().project.skeleton.fps || 30;
}

function zoomBy(s: Services, f: number): void {
  const u = s.ui.get();
  s.ui.set({ view: zoomAt(u.view, u.viewSize, u.viewSize.w / 2, u.viewSize.h / 2, u.view.zoom * f) });
}

/** 등록표를 만든다(화면 없이도 만들 수 있다 — L02 단위 테스트) */
export function buildRegistry(): CommandRegistry<Services> {
  const r = new CommandRegistry<Services>((s, id, args) => {
    s.doc.run(id, args);
  });

  // ---------- 파일(platform) ----------
  r.register('file.new', (s) => s.files.newFile());
  r.register('file.open', async (s, a) => {
    await s.files.open(a.path as string | undefined);
  });
  r.register('file.save', async (s) => {
    await s.files.save();
  });
  r.register('file.saveAs', async (s, a) => {
    await s.files.saveAs(a.path as string | undefined);
  });
  r.register('file.importSpineJson', (s, a) => s.files.importSpineJson(a.path as string | undefined));
  r.register('file.exportSpine', (s, a) => s.files.exportSpine(a as Parameters<typeof s.files.exportSpine>[0]));
  r.register('file.exportPngSequence', (s, a) => s.files.exportPngSequence(a as Parameters<typeof s.files.exportPngSequence>[0]));
  r.register('file.setImagesPath', (s, a) => s.files.setImagesPath(a.path as string | undefined));
  r.register('file.reloadImages', (s) => s.files.reloadImages());
  r.register('file.registerAssociation', (s) => s.files.registerAssociation());
  r.register('app.quit', (s) => s.files.quit());

  // ---------- 편집 ----------
  r.register('edit.delete', (s) => {
    const keys = s.ui.get().selectedKeys;
    if (keys.length && s.doc.get().project.editor.mode === 'animate' && document.activeElement?.closest?.('[data-panel="bottom"]')) {
      if (s.doc.run('key.delete', { keys }).ok) s.ui.set({ selectedKeys: [] });
      return;
    }
    deleteSelection(s);
  });
  r.register('edit.duplicate', (s) => {
    const sel = primary(s);
    if (sel?.kind === 'animation') {
      s.doc.run('animation.duplicate', { name: sel.name });
      return;
    }
    s.ui.notify('warn', '복제는 애니메이션에서만 지원합니다');
  });
  r.register('item.rename', (s, a) => {
    if (a.newName !== undefined) {
      s.doc.run('item.rename', a);
      return;
    }
    const sel = primary(s);
    if (!sel || sel.kind === 'skeleton' || sel.kind === 'image') {
      s.ui.notify('warn', '이름을 바꿀 항목을 고르세요');
      return;
    }
    s.ui.set({ renaming: selKey(sel) });
  });
  r.register('bone.setParentPick', (s) => {
    if (!namesOf(s.ui.get().selection, 'bone').length) {
      s.ui.notify('warn', '부모를 바꿀 본을 먼저 고르세요');
      return;
    }
    s.ui.set({ pickParent: true });
    s.ui.notify('info', '새 부모 본을 클릭하세요 (Esc: 취소)');
  });
  const slotMove = (id: string) => (s: Services, a: Args) => {
    const name = (a.name as string) ?? namesOf(s.ui.get().selection, 'slot')[0];
    if (!name) {
      s.ui.notify('warn', '슬롯을 고르세요');
      return;
    }
    s.doc.run(id, { name });
  };
  r.register('slot.moveUp', slotMove('slot.moveUp'));
  r.register('slot.moveDown', slotMove('slot.moveDown'));
  r.register('select.none', (s) => {
    if (s.ui.get().pickParent) {
      s.ui.set({ pickParent: false });
      return;
    }
    s.ui.set({ selection: [], selectedKeys: [] });
  });

  // ---------- 보기 ----------
  r.register('view.zoomIn', (s) => zoomBy(s, 1.25));
  r.register('view.zoomOut', (s) => zoomBy(s, 0.8));
  r.register('view.zoomReset', (s) => s.ui.set({ view: { ...s.ui.get().view, zoom: 1 } }));
  r.register('view.fit', (s) => {
    const p = s.doc.get().project;
    const b = poseBounds(s.pose.pose(), p.skeleton.bones);
    s.ui.set({ view: fitBounds(s.ui.get().viewSize, b[0], b[1], b[2], b[3]) });
  });
  r.register('view.toggleBones', (s) => s.ui.set({ showBones: !s.ui.get().showBones }));
  r.register('view.toggleImages', (s) => s.ui.set({ showImages: !s.ui.get().showImages }));
  r.register('view.toggleOnion', (s) => s.ui.set({ onion: !s.ui.get().onion }));
  r.register('view.showDopesheet', (s) => s.ui.set({ bottomTab: 'dopesheet' }));
  r.register('view.showGraph', (s) => s.ui.set({ bottomTab: 'graph' }));

  // ---------- 만들기(core, 인자 채우기) ----------
  r.register('bone.create', (s, a) => {
    if (Object.keys(a).length) s.doc.run('bone.create', a);
    else createBone(s);
  });
  r.register('slot.create', (s, a) => {
    if (a.bone) s.doc.run('slot.create', a);
    else createSlot(s);
  });
  r.register('attachment.create', async (s, a) => {
    if (a.slot && a.name) s.doc.run('attachment.create', a);
    else await createAttachment(s, (a.type as AttachmentType) ?? 'region', a.image as string | undefined);
  });
  r.register('constraint.create', (s, a) => {
    if (a.bones || a.bone || a.target || a.animation) s.doc.run('constraint.create', a);
    else createConstraint(s, (a.type as ConstraintType) ?? 'ik');
  });
  r.register('skin.create', (s) => createSimple(s, 'skin'));
  r.register('event.create', (s) => createSimple(s, 'event'));
  r.register('animation.create', (s) => createSimple(s, 'animation'));

  // ---------- 애니메이션 ----------
  r.register('mode.toggle', (s) => {
    s.player.stop();
    s.doc.run('mode.toggle');
  });
  r.register('play.toggle', (s) => s.player.toggle(false));
  r.register('play.reverse', (s) => s.player.toggle(true));
  r.register('play.toggleLoop', (s) => s.ui.set({ loop: !s.ui.get().loop }));
  const step = (dir: number) => (s: Services) => {
    const t = s.pose.currentTime();
    const f = fps(s);
    setTime(s, snapToFrame(t, f) + dir / f);
  };
  r.register('time.prevFrame', step(-1));
  r.register('time.nextFrame', step(1));
  r.register('time.prevKey', (s) => {
    const t = prevKeyTime(keyTimes(activeAnimation(s)), s.pose.currentTime());
    if (t !== null) setTime(s, t);
  });
  r.register('time.nextKey', (s) => {
    const t = nextKeyTime(keyTimes(activeAnimation(s)), s.pose.currentTime());
    if (t !== null) setTime(s, t);
  });
  r.register('time.first', (s) => setTime(s, 0));
  r.register('time.last', (s) => setTime(s, animationDuration(activeAnimation(s))));
  r.register('key.edited', (s, a) => {
    if (a.bones || a.slots) {
      s.doc.run('key.edited', a);
      return;
    }
    const sel = s.ui.get().selection;
    const bones = namesOf(sel, 'bone');
    const slots = namesOf(sel, 'slot');
    s.doc.run('key.edited', { ...(bones.length ? { bones } : {}), ...(slots.length ? { slots } : {}) });
  });
  r.register('key.constrained', (s, a) => {
    s.doc.run('key.constrained', { bones: (a.bones as string[]) ?? namesOf(s.ui.get().selection, 'bone') });
  });
  r.register('key.setCurve', (s, a) => {
    const keys = (a.keys as unknown[]) ?? s.ui.get().selectedKeys;
    if (!keys.length) {
      s.ui.notify('warn', '보간을 바꿀 키를 도프시트에서 고르세요');
      return;
    }
    if (a.curve !== 'bezier') {
      s.doc.run('key.setCurve', { keys, curve: a.curve ?? 'linear' });
      return;
    }
    // 베지어: 키마다 다음 키까지 기본 손잡이
    const anim = activeAnimation(s);
    if (!anim) return;
    s.doc.group('보간 지정', () => {
      for (const k of s.ui.get().selectedKeys) {
        const p = keyPair(anim, k);
        if (p?.next) s.doc.run('key.setCurve', { keys: [k], curve: defaultBezier(p.key, p.next) });
      }
    });
  });

  // ---------- 도구 ----------
  const tools: ToolId[] = ['pose', 'create', 'weights', 'rotate', 'translate', 'scale', 'shear'];
  for (const t of tools) r.register(`tool.${t}`, (s) => s.ui.setTool(t));
  r.register('axis.local', (s) => s.ui.set({ axis: 'local' }));
  r.register('axis.parent', (s) => s.ui.set({ axis: 'parent' }));
  r.register('axis.world', (s) => s.ui.set({ axis: 'world' }));
  r.register('option.compensate', (s) => s.ui.set({ compensate: !s.ui.get().compensate }));

  // ---------- 도움말 ----------
  r.register('help.shortcuts', (s) => s.dialogs.shortcuts());
  r.register('help.about', (s) =>
    s.dialogs.message('KGBone 정보', 'KGBone 0.1.0\n2D 스켈레탈 애니메이션 에디터\n\nRust 핵심(WebAssembly) + React 화면으로 만든 독자 구현입니다.'),
  );
  return r;
}

