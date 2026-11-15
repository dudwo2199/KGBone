// D14: 뷰포트에서 이미지 첨부물 끌어 옮기기(화면 계산과 명령 호출)
import { describe, expect, it } from 'vitest';
import type { Attachment, Project, WorldBone } from '../core/types';
import { createServices } from '../services';
import { emptyProject } from '../state/document';
import { FakeCore, FakePlatform } from '../testing/fakes';
import { DialogService } from '../ui/dialogs/service';
import { AttachmentDrag, attachmentSkin, attachmentTarget, movedPosition, type AttachmentTarget } from './attachmentDrag';
import { axisFrame } from './axis';

const region = (x: number, y: number): Attachment =>
  ({ type: 'region', path: null, x, y, rotation: 0, scaleX: 1, scaleY: 1, width: 32, height: 32, color: 'FFFFFFFF', sequence: null }) as unknown as Attachment;

/** 본 'b'(슬롯 's')에 기본 스킨 'img'(10, 20), 'hat' 스킨에 'img'(1, 2), 메시 'mesh' */
function project(): Project {
  const p = emptyProject();
  p.skeleton.bones = [{ name: 'b', parent: null } as Project['skeleton']['bones'][number]];
  p.skeleton.slots = [{ name: 's', bone: 'b', color: 'FFFFFFFF', dark: null, attachment: 'img', blend: 'normal', visible: true }];
  p.skeleton.skins = [
    { name: 'default', bones: [], constraints: [], attachments: { s: { img: region(10, 20), mesh: { type: 'mesh' } as unknown as Attachment } } },
    { name: 'hat', bones: [], constraints: [], attachments: { s: { img: region(1, 2) } } },
  ];
  return p;
}

const ID: WorldBone = { a: 1, b: 0, c: 0, d: 1, x: 0, y: 0 };
// 90° 회전한 본: 로컬 x축이 월드 위쪽
const R90: WorldBone = { a: 0, b: -1, c: 1, d: 0, x: 50, y: 50 };

describe('D14 뷰포트에서 이미지 첨부물 끌어 옮기기', () => {
  it('u_d14_skin_prefers_active_then_default', () => {
    const p = project();
    expect(attachmentSkin(p, 's', 'img')).toBe('default');
    p.editor.activeSkin = 'hat';
    expect(attachmentSkin(p, 's', 'img')).toBe('hat');
    expect(attachmentSkin(p, 's', 'mesh')).toBe('default');
    expect(attachmentSkin(p, 's', 'none')).toBeNull();
  });

  it('u_d14_target_only_region_in_setup_mode', () => {
    const p = project();
    const t = attachmentTarget(p, [ID], 's', 'img') as AttachmentTarget;
    expect(t).toMatchObject({ slot: 's', name: 'img', skin: undefined, x: 10, y: 20 });
    p.editor.activeSkin = 'hat';
    expect(attachmentTarget(p, [ID], 's', 'img')).toMatchObject({ skin: 'hat', x: 1, y: 2 });
    expect(attachmentTarget(p, [ID], 's', 'mesh')).toMatch(/이미지 첨부물뿐/);
    p.editor.mode = 'animate';
    expect(attachmentTarget(p, [ID], 's', 'img')).toMatch(/셋업 모드/);
  });

  it('u_d14_world_delta_becomes_bone_local', () => {
    const t: AttachmentTarget = { slot: 's', name: 'img', x: 10, y: 20, bone: R90 };
    // 월드 오른쪽 10 = 로컬 -y 10, 월드 위쪽 5 = 로컬 +x 5
    expect(movedPosition(t, 10, 5)).toEqual({ x: 15, y: 10 });
    // Shift: 본 축 한 방향으로만(더 크게 움직인 쪽)
    expect(movedPosition(t, 10, 3, axisFrame('local', R90, R90))).toEqual({ x: 10, y: 10 });
    expect(movedPosition(t, 10, 3, axisFrame('world', R90, R90))).toEqual({ x: 10, y: 10 });
  });

  it('u_d14_drag_runs_attachment_set_in_one_group', () => {
    const core = new FakeCore();
    core.project_ = project();
    const s = createServices(core, new FakePlatform(), new DialogService());
    s.doc.refresh();
    const groups: string[] = [];
    const g = core as unknown as { beginGroup(label: string): void; endGroup(): void };
    g.beginGroup = (label) => void groups.push(`begin:${label}`);
    g.endGroup = () => void groups.push('end');
    const d = new AttachmentDrag(s, 's', 'img');
    d.begin({ x: 100, y: 100 });
    d.move({ x: 103, y: 100 }, { shiftKey: false });
    d.move({ x: 107, y: 96 }, { shiftKey: false });
    d.end();
    expect(groups).toEqual(['begin:그림 이동', 'end']);
    const sets = core.calls.filter((c) => c.id === 'attachment.set');
    expect(sets.at(-1)!.args).toEqual({ slot: 's', name: 'img', props: { x: 17, y: 16 } });
  });

  it('u_d14_click_without_move_changes_nothing_and_animate_mode_warns', () => {
    const core = new FakeCore();
    core.project_ = project();
    const s = createServices(core, new FakePlatform(), new DialogService());
    s.doc.refresh();
    const click = new AttachmentDrag(s, 's', 'img');
    click.begin({ x: 0, y: 0 });
    click.end();
    expect(core.calls).toEqual([]);

    core.project_.editor.mode = 'animate';
    s.doc.refresh();
    const notes: string[] = [];
    const notify = s.ui.notify;
    s.ui.notify = (level, text) => {
      notes.push(text);
      notify(level, text);
    };
    const d = new AttachmentDrag(s, 's', 'img');
    d.begin({ x: 0, y: 0 });
    d.move({ x: 5, y: 0 }, { shiftKey: false });
    d.move({ x: 9, y: 0 }, { shiftKey: false });
    d.end();
    expect(core.calls).toEqual([]);
    expect(notes.filter((n) => n.includes('셋업 모드'))).toHaveLength(1);
  });
});
