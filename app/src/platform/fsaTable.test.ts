import { describe, expect, it } from 'vitest';
import { paths } from './platform';
import { fsaPath, HandleTable, parseFsaPath, type DirLike, type FileLike } from './fsaTable';

// 가짜 폴더 트리
class FakeFile implements FileLike {
  readonly kind = 'file' as const;
  constructor(readonly name: string) {}
}
class FakeDir implements DirLike {
  readonly kind = 'directory' as const;
  dirs = new Map<string, FakeDir>();
  files = new Map<string, FakeFile>();
  constructor(readonly name: string) {}
  async getDirectoryHandle(name: string, o?: { create?: boolean }) {
    let d = this.dirs.get(name);
    if (!d && o?.create) this.dirs.set(name, (d = new FakeDir(name)));
    if (!d) throw new Error('없음');
    return d;
  }
  async getFileHandle(name: string, o?: { create?: boolean }) {
    let f = this.files.get(name);
    if (!f && o?.create) this.files.set(name, (f = new FakeFile(name)));
    if (!f) throw new Error('없음');
    return f;
  }
  async resolve(child: DirLike | FileLike): Promise<string[] | null> {
    if (child === this) return [];
    for (const [n, f] of this.files) if (f === child) return [n];
    for (const [n, d] of this.dirs) {
      const r = await d.resolve(child);
      if (r) return [n, ...r];
    }
    return null;
  }
}

describe('J 브라우저 판 경로·핸들 표', () => {
  it('u_j_fsa_parse_paths', () => {
    expect(parseFsaPath('fsa:/작업/a/./b/../c.png')).toEqual({ root: '작업', parts: ['a', 'c.png'] });
    expect(parseFsaPath('C:\\x')).toBeNull();
    expect(fsaPath('작업', ['images'])).toBe('fsa:/작업/images');
    // 프로젝트 폴더 + imagesPath 상대 경로
    expect(paths.join(paths.dirname('fsa:/작업/proj/a.kgbone'), './images')).toBe('fsa:/작업/proj/images');
    expect(paths.join(paths.dirname('fsa:/작업/proj/a.kgbone'), '../images')).toBe('fsa:/작업/images');
  });

  it('u_j_fsa_table_resolves_files_under_root', async () => {
    const root = new FakeDir('작업');
    const proj = await root.getDirectoryHandle('proj', { create: true });
    const file = await proj.getFileHandle('a.kgbone', { create: true });
    const t = new HandleTable();
    expect(await t.addRoot(root)).toBe('fsa:/작업');
    expect(await t.locate(file)).toBe('fsa:/작업/proj/a.kgbone');
    expect(await t.file('fsa:/작업/proj/a.kgbone')).toBe(file);
    // 없는 파일은 만들기
    const created = await t.file('fsa:/작업/proj/images/head.png', true);
    expect(created.name).toBe('head.png');
    expect(await t.dir('fsa:/작업/proj/images')).toBe(proj.dirs.get('images'));
    // 뿌리 안 폴더를 다시 고르면 같은 뿌리 아래 경로
    expect(await t.addRoot(proj)).toBe('fsa:/작업/proj');
    expect(t.rootNames()).toEqual(['작업']);
  });

  it('u_j_fsa_loose_files_and_same_names', async () => {
    const t = new HandleTable();
    const f = new FakeFile('x.kgbone');
    const p = t.addLoose(f);
    expect(await t.file(p)).toBe(f);
    expect(await t.addRoot(new FakeDir('a'))).toBe('fsa:/a');
    expect(await t.addRoot(new FakeDir('a'))).toBe('fsa:/a-2');
    await expect(t.dir('fsa:/없음/x')).rejects.toThrow('폴더를 찾을 수 없습니다');
  });
});
