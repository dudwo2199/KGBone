// 열린 문서 하나와 그 파일·이미지 폴더를 맡는다. 화면의 FileFlow·ImageLibrary가 하는 일을 창 없이 한다.
// 파일은 허용한 폴더(기본: 서버를 띄운 폴더, 추가: KGBONE_MCP_ROOTS) 안에서만 읽고 쓴다.
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { basename, delimiter, dirname, extname, isAbsolute, join, relative, resolve } from "node:path";
import { readPng } from "./png.mjs";

function allowedRoots() {
  const extra = (process.env.KGBONE_MCP_ROOTS || "").split(delimiter).map((s) => s.trim()).filter(Boolean);
  return [process.cwd(), ...extra].map((p) => resolve(p));
}

/** dir 아래 PNG(하위 폴더 포함): [확장자 없는 상대 경로('/' 구분), 전체 경로] */
function listPngs(dir, prefix = "") {
  const out = [];
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    if (e.isDirectory()) out.push(...listPngs(join(dir, e.name), `${prefix}${e.name}/`));
    else if (e.name.toLowerCase().endsWith(".png")) out.push([prefix + e.name.slice(0, -4), join(dir, e.name)]);
  }
  return out.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
}

export class EditorSession {
  /** 저장할 .kgbone 경로(새 문서·가져온 문서는 null) */
  filePath = null;
  /** imagesPath 상대 경로의 기준 폴더 */
  baseDir = process.cwd();
  /** 등록한 이미지 이름 */
  images = [];

  constructor(core) {
    this.core = core;
    this.roots = allowedRoots();
  }

  /** 허용한 폴더 안의 절대 경로로 바꾼다. 밖이면 오류 */
  resolvePath(path) {
    if (typeof path !== "string" || !path) throw new Error("경로가 없습니다");
    const full = resolve(path); // 상대 경로는 서버를 띄운 폴더 기준
    const inside = this.roots.some((root) => {
      const rel = relative(root, full);
      return rel === "" || (!rel.startsWith("..") && !isAbsolute(rel));
    });
    if (!inside) throw new Error(`허용한 폴더 밖입니다: ${full} (허용: ${this.roots.join(", ")})`);
    return full;
  }

  imagesDir() {
    return resolve(this.baseDir, this.core.project().imagesPath || "./images");
  }

  /** 이미지 폴더를 다시 읽어 크기·알파·PNG 원본을 핵심에 등록한다. 경고 목록을 돌려준다 */
  reloadImages() {
    const dir = this.imagesDir();
    const warnings = [];
    this.images = [];
    if (!existsSync(dir)) return [`이미지 폴더가 없습니다: ${dir}`];
    for (const [name, file] of listPngs(dir)) {
      try {
        const bytes = readFileSync(file);
        const { width, height, alpha } = readPng(bytes);
        this.core.setImage(name, width, height, alpha);
        this.core.addPng(name, bytes);
        this.images.push(name);
        if (!alpha) warnings.push(`알파를 읽지 못했습니다(메시 자동 생성 불가): ${name}`);
      } catch (e) {
        warnings.push(`이미지를 읽지 못했습니다: ${name} (${e.message})`);
      }
    }
    return warnings;
  }

  newDocument() {
    this.core.call({ method: "new" });
    this.filePath = null;
    this.baseDir = process.cwd();
    return this.reloadImages();
  }

  /** .kgbone은 열고, 그 밖(.json)은 Spine JSON 호환 형식으로 가져온다 */
  open(path) {
    const full = this.resolvePath(path);
    const text = readFileSync(full, "utf8");
    const isProject = extname(full).toLowerCase() === ".kgbone";
    const { warnings } = this.core.call({ method: isProject ? "openKgbone" : "importSpineJson", text });
    this.filePath = isProject ? full : null;
    this.baseDir = dirname(full);
    return { kind: isProject ? "kgbone" : "spineJson", path: full, warnings, imageWarnings: this.reloadImages() };
  }

  /** .kgbone으로 저장. path를 주면 그 경로가 새 저장 위치가 된다 */
  save(path) {
    const full = path ? this.resolvePath(path) : this.filePath;
    if (!full) throw new Error("저장할 경로가 없습니다. path를 주세요.");
    if (extname(full).toLowerCase() !== ".kgbone") throw new Error("저장 파일 확장자는 .kgbone이어야 합니다");
    const { text } = this.core.call({ method: "saveKgbone" });
    mkdirSync(dirname(full), { recursive: true });
    writeFileSync(full, text, "utf8");
    this.filePath = full;
    this.baseDir = dirname(full);
    return full;
  }

  /** 내보내기 묶음({name, base64})을 dir에 쓴다 */
  writeBundle(dir, files) {
    const full = this.resolvePath(dir);
    mkdirSync(full, { recursive: true });
    return files.map((f) => {
      const out = join(full, basename(f.name));
      writeFileSync(out, Buffer.from(f.base64, "base64"));
      return out;
    });
  }

  /** 내보내기 파일 이름 앞부분 */
  nameStem() {
    return this.filePath ? basename(this.filePath, extname(this.filePath)) : "skeleton";
  }
}
