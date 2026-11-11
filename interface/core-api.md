# 핵심 모듈 연결 인터페이스 (Rust ↔ 화면)

계산 로직은 `crates/kgbone-core` 한 곳에만 있다. 화면은 이것을 WebAssembly(`crates/kgbone-wasm`)로, 앱 본체(`src-tauri`)는 Rust 라이브러리로 직접 쓴다.

## 1. WebAssembly 함수 (crates/kgbone-wasm, C ABI)
| 함수 | 설명 |
|---|---|
| `kg_alloc(len) -> ptr` / `kg_free(ptr, len)` | 선형 메모리 빌리기·돌려주기 |
| `kg_call(ptr, len) -> outLen` | UTF-8 JSON 요청 처리. 응답은 `kg_result_ptr()`에서 `outLen` 바이트 |
| `kg_set_image(pathPtr, pathLen, w, h, alphaPtr, alphaLen)` | 이미지 크기·알파 등록 |
| `kg_add_png(pathPtr, pathLen, pngPtr, pngLen)` | 내보내기용 PNG 원본 등록 |

화면 쪽 감싸개: `app/src/core/wasm.ts` (`Core` 인터페이스 = `app/src/core/bridge.ts`).

## 2. JSON 요청 (`kgbone_core::api::Session`)
요청 `{"method": 이름, ...}` → 응답 `{"ok": 결과}` 또는 `{"error": "한국어 메시지"}`.

| method | 인자 | 결과 |
|---|---|---|
| `new` | | null |
| `openKgbone` | `text` | `{warnings}` |
| `saveKgbone` | | `{text}` (편집 안 했으면 연 파일과 바이트 동일) |
| `importSpineJson` | `text` | `{warnings}` |
| `exportSpineJson` | `options: {version: "4.3"/"4.2", pretty, imagesPath}` | `{text, warnings}` |
| `exportSpineBundle` | `options: {name, version, pretty}` | `{files: [{name, base64}], warnings}` (json + atlas + png) |
| `exportPngSequence` | `options: {name, animation, skin, fps, scale, width, height}` | `{files, warnings}` |
| `execute` | `id`, `args` | 명령마다 다름 (`interface/commands.json`) |
| `execute` `history.begin` / `history.end` | `{label}` | 되돌리기 묶음 시작·끝 |
| `project` | | Project 전체(기본값 생략 없음, 없는 값은 null) |
| `state` | | `{dirty, canUndo, canRedo, undoLabel, redoLabel}` |
| `pose` | `request: {animation, time, lastTime, skin, physics: "none"/"reset"/dt, looping}` | PoseOutput |
| `resetPhysics` | | null |
| `evalExpr` | `input`, `current` | 숫자 |

## 3. 명령
`interface/commands.json`이 유일한 원본. `scope: core`는 Rust `commands` 모듈이 처리(등록 누락은 Rust 테스트 `f_l02_registry_matches_interface`가 잡는다), `ui`는 화면, `platform`은 화면이 앱 본체를 거쳐 처리.

## 4. 애니메이트 모드 규칙
`editor.mode == "animate"`이고 `editor.activeAnimation`이 있으면, 값 편집 명령(`bone.setTransform`, `slot.setColor`, `slot.setDarkColor`, `slot.setAttachment`, `slot.reorder`(그리기 순서), `mesh.setVertices`(변형), `constraint.set`의 혼합 값)은 셋업 값을 바꾸지 않고 현재 시점(`editor.time`)에 키를 만든다(있으면 고친다).
