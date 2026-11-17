# AI 연결 메시지 (창 ↔ MCP 서버 중계소)

구현: `tools/mcp/link.mjs`(중계소·MCP 서버 쪽), `app/src/ai/agentLink.ts`(창 쪽), `app/src/ai/agentHandler.ts`(창이 처리하는 요청), `tools/mcp/live.mjs`(MCP 도구 → 창 요청).

## 1. 연결
- 먼저 뜬 MCP 서버가 `127.0.0.1:<포트>`(기본 47821, `KGBONE_MCP_PORT`)에 WebSocket 중계소를 연다. 포트가 이미 쓰이면 그 중계소에 `relay`로 붙고, 중계소가 꺼지면 남은 서버 하나가 이어받는다.
- 창은 `ws://127.0.0.1:<포트>/`에 `page`로 붙는다. 브라우저 연결은 Origin이 허용 목록(localhost·127.0.0.1·*.localhost·tauri:·`https://dudwo2199.github.io`·`KGBONE_MCP_ORIGINS`)이어야 하고, 아니면 403. Origin 없는 연결은 `relay`로만 인사할 수 있다.
- 한 프레임에 JSON 하나(글자 프레임).

## 2. 메시지
| 방향 | 메시지 |
|---|---|
| relay → 중계소 | `{type:"hello", role:"relay", name, roots}` (name: MCP initialize의 clientInfo.name, roots: 파일 허용 폴더) |
| relay → 중계소 | `{type:"call", id, method, params}` |
| 중계소 → relay | `{type:"page", connected}`, `{type:"result", id, result \| error}` |
| page → 중계소 | `{type:"hello", role:"page", app:"kgbone"}` |
| 중계소 → page | `{type:"clients", clients:[{id, name}]}`, `{type:"call", id, client, method, params}`, `{type:"replaced"}`(다른 창이 붙음) |
| page → 중계소 | `{type:"result", id, result \| error}` |
| page → 중계소 | `{type:"fs", id, op, ...}` → `{type:"fs-result", id, result \| error}`; 폴더가 바뀌면 `{type:"fs-changed", dir}` |

창은 `call`을 받은 차례대로 하나씩 처리한다. 응답이 120초 안에 없으면 MCP 서버가 실패로 돌려준다.

## 3. 창이 처리하는 call (method)
| method | params | result |
|---|---|---|
| `core` | `{request}` — api.rs 요청 중 `execute`, `project`, `state`, `pose`, `exportPngSequence`, `exportSpineBundle` | 핵심 응답의 ok 값 |
| `doc.new` | `{}` | `{imageWarnings}` (저장 확인에서 취소하면 오류) |
| `doc.open` | `{path, kind: "kgbone" \| "spineJson"}` | `{warnings, imageWarnings}` |
| `doc.save` | `{path \| null}` | `{path}` |
| `doc.info` | `{}` | `{filePath, imagesDir, images, nameStem}` |
| `images.reload` | `{}` | `{warnings}` |
| `ui.run` | `{id, args}` — commands.json의 scope=ui 명령 | `null` |
| `ui.screenshot` | `{}` | `{base64, width, height}` (뷰포트 PNG) |

## 4. 디스크 경로와 fs 작업
- MCP 서버가 연 이 PC의 파일은 창 안에서 `disk:` + 절대 경로로 쓴다(`app/src/platform/diskPlatform.ts`). Tauri 판은 앞머리를 떼고 직접 읽는다.
- 브라우저 판은 `fs` 메시지로 중계소에 맡긴다: `readText {path}`, `writeText {path, text}`, `readBinary {path}`(base64), `writeBinary {path, base64}`, `listImages {dir}`(`[{name, width, height, base64}]`), `watch {dir}`, `unwatch {dir}`.
- 중계소는 붙어 있는 모든 MCP 서버의 허용 폴더 안의 경로만 처리한다.
