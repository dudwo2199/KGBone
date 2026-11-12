// File System Access 핸들 표: 고른 폴더(뿌리)를 이름으로 등록하고 가짜 경로 "fsa:/<뿌리>/<하위 경로>"로 찾는다.
export const FSA_PREFIX = 'fsa:/';

/** 표가 쓰는 핸들의 최소 모양(테스트에서 가짜로 바꿀 수 있게) */
export interface DirLike {
  readonly kind: 'directory';
  readonly name: string;
  getDirectoryHandle(name: string, opts?: { create?: boolean }): Promise<DirLike>;
  getFileHandle(name: string, opts?: { create?: boolean }): Promise<FileLike>;
  resolve(child: DirLike | FileLike): Promise<string[] | null>;
}
export interface FileLike {
  readonly kind: 'file';
  readonly name: string;
}

export function isFsaPath(p: string): boolean {
  return p.startsWith(FSA_PREFIX);
}

/** "fsa:/뿌리/a/b.png" → { root: '뿌리', parts: ['a', 'b.png'] } ('.', '..' 정리) */
export function parseFsaPath(p: string): { root: string; parts: string[] } | null {
  if (!isFsaPath(p)) return null;
  const segs = p.slice(FSA_PREFIX.length).split(/[\\/]/).filter((s) => s && s !== '.');
  const out: string[] = [];
  for (const s of segs) {
    if (s === '..') {
      if (out.length > 1) out.pop();
    } else out.push(s);
  }
  if (!out.length) return null;
  return { root: out[0], parts: out.slice(1) };
}

export function fsaPath(root: string, parts: string[]): string {
  return FSA_PREFIX + [root, ...parts].join('/');
}

export class HandleTable<D extends DirLike = DirLike, F extends FileLike = FileLike> {
  private roots = new Map<string, D>();
  /** 폴더 밖에서 고른 파일(같은 폴더를 모름) */
  private loose = new Map<string, F>();
  private looseCount = 0;

  rootNames(): string[] {
    return [...this.roots.keys()];
  }

  /** 폴더를 뿌리로 등록하고 경로를 돌려준다. 이미 등록한 뿌리 안이면 그 아래 경로 */
  async addRoot(dir: D): Promise<string> {
    const inside = await this.locate(dir);
    if (inside) return inside;
    let name = dir.name || 'folder';
    for (let i = 2; this.roots.has(name) && this.roots.get(name) !== dir; i++) name = `${dir.name}-${i}`;
    this.roots.set(name, dir);
    return fsaPath(name, []);
  }

  /** 등록된 뿌리 아래에 있으면 그 경로, 아니면 null */
  async locate(h: D | F): Promise<string | null> {
    for (const [name, root] of this.roots) {
      try {
        const parts = await root.resolve(h);
        if (parts) return fsaPath(name, parts);
      } catch {
        // 다른 뿌리 확인
      }
    }
    return null;
  }

  addLoose(f: F): string {
    const p = fsaPath(`~파일${++this.looseCount}`, [f.name]);
    this.loose.set(p, f);
    return p;
  }

  async dir(path: string, create = false): Promise<D> {
    const parsed = parseFsaPath(path);
    const root = parsed && this.roots.get(parsed.root);
    if (!parsed || !root) throw new Error(`폴더를 찾을 수 없습니다: ${path}`);
    let d: DirLike = root;
    for (const s of parsed.parts) d = await d.getDirectoryHandle(s, { create });
    return d as D;
  }

  async file(path: string, create = false): Promise<F> {
    const l = this.loose.get(path);
    if (l) return l;
    const parsed = parseFsaPath(path);
    if (!parsed || !parsed.parts.length) throw new Error(`파일을 찾을 수 없습니다: ${path}`);
    const dirPath = fsaPath(parsed.root, parsed.parts.slice(0, -1));
    const d = await this.dir(dirPath, create);
    return (await d.getFileHandle(parsed.parts[parsed.parts.length - 1], { create })) as F;
  }

  /** 마지막으로 쓴 뿌리(대화상자 시작 폴더) */
  lastRoot(): D | undefined {
    return [...this.roots.values()].pop();
  }
}
