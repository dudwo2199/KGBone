// MCP 도구 목록과 처리기. 편집은 모두 핵심의 명령(interface/commands.json, scope: core)으로 한다.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { pick, summarize } from "./inspect.mjs";

const commandsFile = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "interface", "commands.json");
const { commands: COMMANDS, types: COMMAND_TYPES } = JSON.parse(readFileSync(commandsFile, "utf8"));
const byId = new Map(COMMANDS.map((c) => [c.id, c]));

/** 한 번에 돌려주는 글자 수 한도(넘으면 자른다) */
const TEXT_LIMIT = 60000;

const text = (value) => {
  let s = typeof value === "string" ? value : JSON.stringify(value);
  if (s.length > TEXT_LIMIT) s = `${s.slice(0, TEXT_LIMIT)}\n… (잘림: 전체 ${s.length}자. doc_inspect의 path로 범위를 좁히세요)`;
  return { type: "text", text: s };
};

/** core 명령인지 확인한다. 화면·파일 명령은 어떤 도구를 쓸지 알려 준다 */
function coreCommand(id) {
  const c = byId.get(id);
  if (!c) throw new Error(`없는 명령입니다: ${id} (command_list로 찾아보세요)`);
  if (c.scope !== "core") {
    const hint = c.scope === "platform" ? "파일 일은 doc_open, doc_save, export_* 도구를 쓰세요." : "화면 전용 명령이라 창 없는 서버에서는 쓸 수 없습니다.";
    throw new Error(`${id}는 ${c.scope} 명령입니다. ${hint}`);
  }
  return c;
}

/** 명령 실행 뒤에 따라야 하는 일(이미지 폴더가 바뀌면 다시 읽기) */
function afterCommand(session, id) {
  return id === "project.setImagesPath" ? session.reloadImages() : [];
}

const str = { type: "string" };
const num = { type: "number" };
const schema = (properties = {}, required = []) => ({ type: "object", properties, required, additionalProperties: false });

export const TOOLS = [
  {
    name: "doc_new",
    description: "빈 문서를 새로 만든다(root 본 하나). 저장하지 않은 변경은 버린다.",
    inputSchema: schema(),
    run: (s) => [text({ imageWarnings: s.newDocument(), state: s.core.state() })],
  },
  {
    name: "doc_open",
    description: ".kgbone 파일을 열거나, 그 밖의 파일(.json)을 Spine JSON 호환 형식으로 가져온다. 같은 폴더 기준 imagesPath의 PNG도 함께 읽는다. 저장하지 않은 변경은 버린다.",
    inputSchema: schema({ path: { ...str, description: "파일 경로(상대 경로는 서버를 띄운 폴더 기준)" } }, ["path"]),
    run: (s, a) => [text(s.open(a.path))],
  },
  {
    name: "doc_save",
    description: ".kgbone으로 저장한다. path를 생략하면 연 파일에 덮어쓴다. 편집하지 않았으면 연 파일과 바이트가 같다.",
    inputSchema: schema({ path: { ...str, description: "저장할 .kgbone 경로(생략하면 연 파일)" } }),
    run: (s, a) => [text({ saved: s.save(a.path), state: s.core.state() })],
  },
  {
    name: "doc_inspect",
    description:
      "문서를 읽는다. path가 없으면 요약(본·슬롯·스킨·애니메이션 이름, 되돌리기 상태, 파일·이미지)을, 있으면 Project JSON의 그 부분을 돌려준다. " +
      "path 예: skeleton.bones.root, skeleton.skins.default.attachments, skeleton.animations.walk.bones.arm (배열은 번호나 name으로 고른다).",
    inputSchema: schema({ path: { ...str, description: "점으로 이은 경로" } }),
    run: (s, a) => {
      const project = s.core.project();
      return [text(a.path ? pick(project, a.path) : summarize(project, s.core.state(), s))];
    },
  },
  {
    name: "command_list",
    description: "쓸 수 있는 편집 명령(id, 설명, 인자)을 찾는다. filter는 id·설명에 들어 있는 글자. 인자 이름 끝의 ?는 생략 가능하다는 뜻이다.",
    inputSchema: schema({ filter: { ...str, description: "예: bone, key, mesh, 스킨" } }),
    run: (_s, a) => {
      const f = (a.filter || "").toLowerCase();
      const list = COMMANDS.filter((c) => c.scope === "core" && (!f || c.id.toLowerCase().includes(f) || c.label.toLowerCase().includes(f)));
      return [text({ commands: list.map((c) => ({ id: c.id, label: c.label, args: c.args })), types: COMMAND_TYPES })];
    },
  },
  {
    name: "command_run",
    description:
      "편집 명령 하나를 실행한다(되돌리기 한 번). 애니메이트 모드(mode.set)에서 활성 애니메이션이 있으면 값 편집 명령은 셋업 값 대신 현재 시점(time.set)에 키를 만든다. " +
      "되돌리기는 edit.undo, 다시 실행은 edit.redo.",
    inputSchema: schema({ id: { ...str, description: "명령 id(command_list)" }, args: { type: "object", description: "명령 인자" } }, ["id"]),
    run: (s, a) => {
      coreCommand(a.id);
      const result = s.core.execute(a.id, a.args ?? {});
      const imageWarnings = afterCommand(s, a.id);
      return [text({ result, state: s.core.state(), ...(imageWarnings.length ? { imageWarnings } : {}) })];
    },
  },
  {
    name: "command_batch",
    description: "명령 여러 개를 차례로 실행하고 되돌리기 한 번으로 묶는다. 중간에 하나가 실패하면 앞서 실행한 것까지 되돌리고 몇 번째가 왜 실패했는지 알려 준다.",
    inputSchema: schema(
      {
        label: { ...str, description: "되돌리기 목록에 보일 이름" },
        commands: { type: "array", items: schema({ id: str, args: { type: "object" } }, ["id"]), minItems: 1 },
      },
      ["commands"],
    ),
    run: (s, a) => {
      for (const c of a.commands) coreCommand(c.id);
      const label = a.label || "여러 명령";
      const results = [];
      s.core.execute("history.begin", { label });
      let failure = null;
      for (const [i, c] of a.commands.entries()) {
        try {
          results.push(s.core.execute(c.id, c.args ?? {}));
        } catch (e) {
          failure = { index: i, id: c.id, message: e.message };
          break;
        }
      }
      s.core.execute("history.end");
      if (failure) {
        // 묶음에 실제로 들어간 것이 있을 때만 되돌린다
        const st = s.core.state();
        const rolledBack = failure.index > 0 && st.canUndo && st.undoLabel === label;
        if (rolledBack) s.core.execute("edit.undo");
        throw new Error(`${failure.index + 1}번째 명령(${failure.id}) 실패: ${failure.message}${rolledBack ? " — 앞선 명령은 되돌렸습니다." : ""}`);
      }
      const imageWarnings = a.commands.some((c) => c.id === "project.setImagesPath") ? s.reloadImages() : [];
      return [text({ results, state: s.core.state(), ...(imageWarnings.length ? { imageWarnings } : {}) })];
    },
  },
  {
    name: "pose",
    description: "어느 시점의 계산된 뼈대를 숫자로 읽는다: 본마다 월드 행렬(a b c d)·위치(x y)와 로컬 값, 그려지는 첨부물 순서, 그 시점에 난 이벤트.",
    inputSchema: schema({
      animation: { ...str, description: "애니메이션 이름(생략하면 셋업 포즈)" },
      time: { ...num, description: "초(기본 0)" },
      skin: { ...str, description: "스킨 이름(생략하면 기본 스킨)" },
    }),
    run: (s, a) => {
      const out = s.core.call({ method: "pose", request: { animation: a.animation ?? null, time: a.time ?? 0, skin: a.skin ?? null, physics: "reset", looping: false } });
      const names = s.core.project().skeleton.bones.map((b) => b.name);
      return [
        text({
          bones: out.bones.map((w, i) => ({ name: names[i], active: out.active[i], world: w, local: out.locals[i] })),
          draw: out.draw.map((d) => ({ slot: d.slot, attachment: d.attachment, image: d.image })),
          events: out.events,
        }),
      ];
    },
  },
  {
    name: "render",
    description:
      "어느 시점의 모습을 PNG 그림으로 돌려준다(편집 결과를 눈으로 확인). 이미지가 붙은 첨부물만 그려지고 본 자체는 그려지지 않는다 — 본 위치는 pose로 본다. " +
      "시점은 fps 간격의 가장 가까운 프레임으로 맞춘다.",
    inputSchema: schema({
      animation: { ...str, description: "애니메이션 이름(생략하면 셋업 포즈)" },
      time: { ...num, description: "초(기본 0)" },
      skin: { ...str, description: "스킨 이름(생략하면 편집 중인 스킨)" },
      scale: { ...num, description: "배율(기본 1)" },
      width: { ...num, description: "픽셀 너비(생략하면 셋업 포즈에 맞춤)" },
      height: { ...num, description: "픽셀 높이(생략하면 셋업 포즈에 맞춤)" },
    }),
    run: (s, a) => {
      const project = s.core.project();
      const fps = project.skeleton.fps > 0 ? project.skeleton.fps : 30;
      const bundle = s.core.call({
        method: "exportPngSequence",
        options: {
          name: "frame",
          animation: a.animation ?? null,
          skin: a.skin ?? project.editor.activeSkin,
          fps,
          scale: a.scale ?? 1,
          ...(a.width > 0 ? { width: a.width } : {}),
          ...(a.height > 0 ? { height: a.height } : {}),
        },
      });
      if (!bundle.files.length) throw new Error("그릴 프레임이 없습니다");
      const index = Math.min(bundle.files.length - 1, Math.max(0, Math.round((a.time ?? 0) * fps)));
      const note = { animation: a.animation ?? null, time: index / fps, frame: index, frames: bundle.files.length, warnings: bundle.warnings };
      return [{ type: "image", data: bundle.files[index].base64, mimeType: "image/png" }, text(note)];
    },
  },
  {
    name: "images_reload",
    description: "이미지 폴더(imagesPath)를 다시 읽는다. 폴더에 PNG를 넣거나 바꾼 뒤에 부른다.",
    inputSchema: schema(),
    run: (s) => {
      const warnings = s.reloadImages();
      return [text({ imagesDir: s.imagesDir(), images: s.images, warnings })];
    },
  },
  {
    name: "export_spine",
    description: "Spine JSON 호환 파일 + 아틀라스 + 텍스처 PNG를 폴더에 내보낸다.",
    inputSchema: schema(
      {
        dir: { ...str, description: "내보낼 폴더" },
        name: { ...str, description: "파일 이름 앞부분(생략하면 문서 이름)" },
        version: { enum: ["4.3", "4.2"], description: "형식 버전(기본 4.3)" },
        pretty: { type: "boolean", description: "들여쓰기(기본 true)" },
      },
      ["dir"],
    ),
    run: (s, a) => {
      const options = { name: a.name || s.nameStem(), version: a.version ?? "4.3", pretty: a.pretty ?? true, imagesPath: s.core.project().imagesPath };
      const bundle = s.core.call({ method: "exportSpineBundle", options });
      return [text({ files: s.writeBundle(a.dir, bundle.files), warnings: bundle.warnings })];
    },
  },
  {
    name: "export_png_sequence",
    description: "애니메이션을 프레임마다 PNG로 폴더에 내보낸다.",
    inputSchema: schema(
      {
        dir: { ...str, description: "내보낼 폴더" },
        animation: { ...str, description: "애니메이션 이름(생략하면 셋업 포즈 한 장)" },
        skin: { ...str, description: "스킨 이름(생략하면 편집 중인 스킨)" },
        fps: { ...num, description: "초당 프레임(생략하면 문서 값)" },
        scale: { ...num, description: "배율(기본 1)" },
      },
      ["dir"],
    ),
    run: (s, a) => {
      const project = s.core.project();
      const bundle = s.core.call({
        method: "exportPngSequence",
        options: { name: s.nameStem(), animation: a.animation ?? null, skin: a.skin ?? project.editor.activeSkin, fps: a.fps ?? project.skeleton.fps ?? 30, scale: a.scale ?? 1 },
      });
      return [text({ files: s.writeBundle(a.dir, bundle.files), warnings: bundle.warnings })];
    },
  },
];

const toolByName = new Map(TOOLS.map((t) => [t.name, t]));

/** tools/list 응답용(처리기 뺀 정의) */
export function listTools() {
  return TOOLS.map(({ name, description, inputSchema }) => ({ name, description, inputSchema }));
}

/** tools/call 처리. 실패는 예외 대신 isError 결과로 돌려준다(MCP 규약) */
export function callTool(session, name, args) {
  const tool = toolByName.get(name);
  if (!tool) return { content: [text(`없는 도구입니다: ${name}`)], isError: true };
  try {
    return { content: tool.run(session, args ?? {}) };
  } catch (e) {
    return { content: [text(e instanceof Error ? e.message : String(e))], isError: true };
  }
}
