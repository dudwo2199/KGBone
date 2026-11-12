import { describe, expect, it, vi } from 'vitest';
import { DocStore } from '../state/document';
import { ImageLibrary } from '../state/images';
import { FakeCore, FakePlatform } from '../testing/fakes';
import type { AppDialogs, SaveChoice } from './dialogs';
import { FileFlow, relativePath } from './fileFlow';

function setup(choice: SaveChoice) {
  const core = new FakeCore();
  const platform = new FakePlatform();
  const notify = vi.fn();
  const doc = new DocStore(core, notify, '/samples');
  const images = new ImageLibrary(platform, core, notify, async () => undefined);
  const confirmSave = vi.fn(async () => choice);
  const dialogs = { confirmSave, report: vi.fn(async () => {}), message: vi.fn(async () => {}) } as unknown as AppDialogs;
  const flow = new FileFlow({ core, platform, doc, images, dialogs, notify, onDocumentReplaced: () => {} });
  return { core, platform, doc, flow, confirmSave };
}

describe('J10 저장 안 한 변경 확인', () => {
  it('u_j10_clean_document_closes_without_asking', async () => {
    const t = setup('cancel');
    expect(await t.flow.confirmDiscard()).toBe(true);
    expect(t.confirmSave).not.toHaveBeenCalled();
  });

  it('u_j10_dirty_document_asks_and_cancel_keeps_it', async () => {
    const t = setup('cancel');
    t.doc.run('bone.create');
    expect(t.doc.get().docState.dirty).toBe(true);
    expect(await t.flow.confirmDiscard()).toBe(false);
    expect(t.confirmSave).toHaveBeenCalledTimes(1);
  });

  it('u_j10_dirty_discard_and_save', async () => {
    const d = setup('discard');
    d.doc.run('bone.create');
    expect(await d.flow.confirmDiscard()).toBe(true);
    expect(d.core.saved).toBe(0);

    const s = setup('save');
    s.doc.run('bone.create');
    expect(await s.flow.confirmDiscard()).toBe(true);
    expect(s.core.saved).toBe(1);
    expect(s.platform.files.has('/p/new.kgbone')).toBe(true);
    expect(s.doc.get().filePath).toBe('/p/new.kgbone');
  });

  it('u_j10_new_file_with_dirty_asks', async () => {
    const t = setup('cancel');
    t.doc.run('bone.create');
    await t.flow.newFile();
    expect(t.confirmSave).toHaveBeenCalled();
    expect(t.doc.get().docState.dirty).toBe(true);
  });
});

describe('J 파일 경로', () => {
  it('u_j_save_adds_extension', async () => {
    const t = setup('cancel');
    t.platform.saveAnswer = '/p/이름';
    await t.flow.saveAs();
    expect(t.platform.files.has('/p/이름.kgbone')).toBe(true);
  });

  it('u_j_relative_path', () => {
    expect(relativePath('/samples', '/samples/images')).toBe('./images');
    expect(relativePath('/samples/kgbone', '/samples/images')).toBe('../images');
    expect(relativePath('C:\\a\\b', 'D:\\img')).toBe('D:\\img');
  });
});

describe('J08 이미지 폴더 자동 갱신', () => {
  it('u_j08_watch_event_refreshes_list', async () => {
    vi.useFakeTimers();
    const core = new FakeCore();
    const platform = new FakePlatform();
    const lib = new ImageLibrary(platform, core, vi.fn(), async () => undefined);
    platform.images = [{ name: 'head', url: 'u1', width: 10, height: 20 }];
    await lib.open('/img');
    expect(lib.get().entries.map((e) => e.name)).toEqual(['head']);
    expect(core.images).toEqual([{ path: 'head', w: 10, h: 20 }]);
    platform.images = [...platform.images, { name: 'arm', url: 'u2', width: 4, height: 4 }];
    platform.watchCb!();
    await vi.advanceTimersByTimeAsync(300);
    expect(lib.get().entries.map((e) => e.name)).toEqual(['head', 'arm']);
    vi.useRealTimers();
  });
});
