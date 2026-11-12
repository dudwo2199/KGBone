// 메모리 가상 파일 시스템(브라우저·자동 테스트용). 경로는 '/'로 시작하는 절대 경로.
type Entry = { kind: 'text'; text: string } | { kind: 'bin'; data: Uint8Array } | { kind: 'url'; url: string };

export function normPath(p: string): string {
  const parts = p.replace(/\\/g, '/').split('/');
  const out: string[] = [];
  for (const s of parts) {
    if (!s || s === '.') continue;
    if (s === '..') out.pop();
    else out.push(s);
  }
  return '/' + out.join('/');
}

export class VirtualFs {
  private files = new Map<string, Entry>();
  private dirs = new Set<string>(['/']);
  private watchers = new Set<{ dir: string; cb: () => void }>();

  private touch(path: string): void {
    let d = path;
    for (;;) {
      const i = d.lastIndexOf('/');
      d = i <= 0 ? '/' : d.slice(0, i);
      this.dirs.add(d);
      if (d === '/') break;
    }
    for (const w of [...this.watchers]) if (path.startsWith(w.dir === '/' ? '/' : w.dir + '/')) w.cb();
  }

  mkdir(path: string): void {
    const p = normPath(path);
    this.dirs.add(p);
    this.touch(p + '/.');
  }

  writeText(path: string, text: string): void {
    const p = normPath(path);
    this.files.set(p, { kind: 'text', text });
    this.touch(p);
  }
  writeBinary(path: string, data: Uint8Array): void {
    const p = normPath(path);
    this.files.set(p, { kind: 'bin', data });
    this.touch(p);
  }
  /** 내용 대신 URL로 들고 있는 파일(번들된 시험 이미지) */
  addUrl(path: string, url: string): void {
    const p = normPath(path);
    this.files.set(p, { kind: 'url', url });
    this.touch(p);
  }

  exists(path: string): boolean {
    return this.files.has(normPath(path));
  }
  isDir(path: string): boolean {
    return this.dirs.has(normPath(path));
  }

  async readText(path: string): Promise<string> {
    const e = this.files.get(normPath(path));
    if (!e) throw new Error(`파일이 없습니다: ${path}`);
    if (e.kind === 'text') return e.text;
    if (e.kind === 'bin') return new TextDecoder().decode(e.data);
    return await (await fetch(e.url)).text();
  }

  async readBinary(path: string): Promise<Uint8Array> {
    const e = this.files.get(normPath(path));
    if (!e) throw new Error(`파일이 없습니다: ${path}`);
    if (e.kind === 'bin') return e.data;
    if (e.kind === 'text') return new TextEncoder().encode(e.text);
    return new Uint8Array(await (await fetch(e.url)).arrayBuffer());
  }

  /** 표시·불러오기용 URL(바이너리는 Blob URL) */
  urlOf(path: string): string | null {
    const e = this.files.get(normPath(path));
    if (!e) return null;
    if (e.kind === 'url') return e.url;
    const data = e.kind === 'bin' ? e.data : new TextEncoder().encode(e.text);
    return URL.createObjectURL(new Blob([data.slice().buffer as ArrayBuffer]));
  }

  /** dir 아래 모든 파일(하위 폴더 포함) */
  list(dir = '/'): string[] {
    const d = normPath(dir);
    const prefix = d === '/' ? '/' : d + '/';
    return [...this.files.keys()].filter((p) => p.startsWith(prefix)).sort();
  }

  listDirs(): string[] {
    return [...this.dirs].sort();
  }

  watch(dir: string, cb: () => void): () => void {
    const w = { dir: normPath(dir), cb };
    this.watchers.add(w);
    return () => this.watchers.delete(w);
  }
}
