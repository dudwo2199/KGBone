// 창 연결 중의 세션: 도구는 창 없는 세션(EditorSession)과 같은 모양으로 부르고, 실제 일은 열린 KGBone 창이 한다.
// 핵심 요청은 창의 핵심으로, 파일 열기·저장은 창의 파일 흐름으로 보낸다(창의 경로는 'disk:' + 절대 경로).
// 파일 경로 확인(허용 폴더)과 내보내기 파일 쓰기는 이 서버가 한다.
import { extname } from "node:path";

const DISK = "disk:";
const strip = (p) => (typeof p === "string" && p.startsWith(DISK) ? p.slice(DISK.length) : p);

export class LiveSession {
  where = "window";

  /**
   * @param link Link(창 연결)
   * @param disk EditorSession(경로 확인·묶음 쓰기에 쓴다)
   */
  constructor(link, disk) {
    this.link = link;
    this.disk = disk;
    const call = (request) => link.call("core", { request });
    this.core = {
      call,
      execute: (id, args = {}) => call({ method: "execute", id, args }),
      project: () => call({ method: "project" }),
      state: () => call({ method: "state" }),
    };
  }

  /** 화면 명령(scope: ui) 실행 */
  runUi(id, args) {
    return this.link.call("ui.run", { id, args: args ?? {} });
  }

  async screenshot() {
    return this.link.call("ui.screenshot", {});
  }

  async newDocument() {
    const r = await this.link.call("doc.new", {});
    return r.imageWarnings ?? [];
  }

  async open(path) {
    const full = this.disk.resolvePath(path);
    const isProject = extname(full).toLowerCase() === ".kgbone";
    const r = await this.link.call("doc.open", { path: DISK + full, kind: isProject ? "kgbone" : "spineJson" });
    return { kind: isProject ? "kgbone" : "spineJson", path: full, warnings: r.warnings ?? [], imageWarnings: r.imageWarnings ?? [] };
  }

  async save(path) {
    let target = null;
    if (path) {
      const full = this.disk.resolvePath(path);
      if (extname(full).toLowerCase() !== ".kgbone") throw new Error("저장 파일 확장자는 .kgbone이어야 합니다");
      target = DISK + full;
    }
    const r = await this.link.call("doc.save", { path: target });
    return strip(r.path);
  }

  async reloadImages() {
    const r = await this.link.call("images.reload", {});
    return r.warnings ?? [];
  }

  async info() {
    const r = await this.link.call("doc.info", {});
    return { where: this.where, filePath: strip(r.filePath), imagesDir: strip(r.imagesDir), images: r.images ?? [] };
  }

  async imagesDir() {
    return (await this.info()).imagesDir;
  }

  get images() {
    return undefined;
  }

  writeBundle(dir, files) {
    return this.disk.writeBundle(dir, files);
  }

  async nameStem() {
    return (await this.link.call("doc.info", {})).nameStem || "skeleton";
  }
}
