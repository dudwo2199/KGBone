# 이어서 작업하기 (다른 PC용 인수인계)

마지막 갱신: 2026-11-17(9단계 마무리). 작업 명세 원본은 저장소 밖의 `KGBone 제작 프롬프트.txt`(0~9단계)이고, 규칙은 `CLAUDE.md`에 모두 옮겨 두었다.

## 1. 지금 어디까지 했나
| 단계 | 상태 |
|---|---|
| 0~8단계 | 끝남 (docs/LOG.md에 단계별 끝난 시각) |
| 9단계 마무리 | 끝남(2026-11-17): 최종 보고 `docs/REPORT.md`(시간·규모·테스트 787개·실행 파일·3분 시연 순서) |
| 9단계 뒤 추가 | 웹 판(설치형 웹 앱, GitHub Pages), MCP 서버, 뷰포트 그림 끌어 옮기기(D14), README — 끝남 |
| 유니티 런타임 1단계 | 끝남(배포 전, 아래 1-1) |

- 테스트: Rust 165개(핵심 156 + 앱 본체 9, 앱 본체는 CI에서만 실행), 화면 단위 66개, MCP 서버 18개, 화면 흐름(Playwright) 11개 — 모두 통과.
- 코드: Rust 약 17,600줄 / TypeScript 약 8,700줄, 저장소 파일 약 270개.
- 아직 안 되는 것·미확인 표기: `docs/GAPS.md`.

### 1-1. 유니티 런타임 1단계 (끝남, 배포 전)
- 설계 `docs/superpowers/specs/2026-11-16-unity-runtime-stage1-design.md`, 계획 `docs/superpowers/plans/2026-11-16-unity-runtime-stage1.md`, 명세 SPEC.md M절.
- 패키지 `unity/com.kgbone.runtime/`: 계산부(C#)가 KGBone 핵심과 같은지 정답으로 비교(`npm run unity:test`, CI 워크플로 `unity-core.yml`). 유니티 EditMode(`npm run unity:editmode`).
- 확인용 프로젝트 `unity/KGBoneSandbox/`: `npm run unity:sandbox -- <폴더>`로 파일을 넣고, 메뉴 KGBone > 화면 찍기(또는 배치 모드 `-executeMethod KGBoneSandbox.SandboxTools.CaptureBatch`)로 `Logs/capture-*.png`를 만든다.
- 이 PC: Unity 6000.3.17f1, .NET SDK 10. `dotnet test`는 스마트 앱 컨트롤에 막히지 않는다.
- 2단계(제약 5종, 물리, 월드 움직임 반영 옵션) 끝남: 설계 `docs/superpowers/specs/2026-11-17-unity-runtime-stage2-design.md`, 계획 `docs/superpowers/plans/2026-11-17-unity-runtime-stage2.md`, SPEC M13~M16. 정답 912개(물리 연속 포함), EditMode 274개 통과(1개 건너뜀).
- 다음 단계 후보: 3단계(클리핑·시퀀스·섞기·스킨 바꾸기·본 덮어쓰기). 미확인 사항은 GAPS G-26~G-28.
- **배포 전 할 일: 저장소 라이선스 정하기.** 유니티 패키지가 완성되면 배포(버전 태그·설치 주소 안내) 전에 라이선스 파일을 추가한다(사용자 결정, 2026-11-16). 지금은 라이선스가 없어 공개 저장소지만 남이 코드를 가져다 쓸 수 없는 상태다.
- 시험용 골렘(유료 에셋 미리보기에서 잘라 만든 것)은 원래 폴더의 `local/golem/`에만 있고 커밋하지 않는다(`local/`은 로컬 git 제외 목록에 있음).

## 2. 새 PC 준비
1. **Node.js 24** 설치.
2. **Rust**(rustup) 설치 후 타깃 추가:
   ```bash
   rustup target add wasm32-unknown-unknown wasm32-wasip1
   ```
3. Windows에서 Tauri 실행 파일까지 직접 만들려면 **Visual Studio C++ 빌드 도구**와 WebView2(Windows 11 기본 포함)가 필요하다.
4. 저장소 받기와 의존성 설치:
   ```bash
   git clone https://github.com/dudwo2199/KGBone.git
   cd KGBone
   npm install
   ```

## 3. 자주 쓰는 명령
```bash
npm run wasm        # Rust 핵심 → app/public/kgbone.wasm
npm run dev         # 개발 서버 http://localhost:1420 (주소 끝에 ?mem=1 이면 메모리 파일 판)
npm run app         # 브라우저 판 실행: 빌드 후 Edge 앱 창으로 열기 (KGBone 실행.cmd 와 같음)
npm run test:rust   # Rust 테스트(WASI, Node로 실행)
npm run test:ui     # 화면 단위 테스트
npm run test:mcp    # MCP 서버 테스트
npm run test:e2e    # 화면 흐름 테스트(설치된 Edge 사용)
npm run unity:test  # 유니티 런타임 계산부 테스트(dotnet)
npm run unity:editmode  # 유니티 EditMode 테스트(유니티 6000.3)
npm run wasm:fetch  # Rust가 없는 PC: 배포된 kgbone.wasm 받기(4-2장)
npm run icons       # 아이콘 다시 만들기(tools/icon.png, app/public/icons)
npm run build:exe   # Tauri 실행 파일(src-tauri/target/release/KGBone.exe) — 아래 4장 참고
```

## 4. 실행 파일(.exe)
- 원래 PC는 Windows **스마트 앱 컨트롤**이 켜져 있어 서명 없는 exe의 빌드(빌드 스크립트)와 실행이 막혔다. 그래서:
  - `kgbone-core`·`kgbone-wasm`은 외부 크레이트 0개로 만들었고, Rust 테스트는 wasm32-wasip1 + Node로 돈다.
  - exe는 **GitHub Actions**(`.github/workflows/build-windows.yml`)가 main에 push할 때마다 빌드해 `build-output` 브랜치에 올린다.
- 받는 법:
  ```bash
  git fetch origin build-output
  git show origin/build-output:build-info.txt
  git restore --source origin/build-output --worktree -- KGBone.exe   # 또는: git show origin/build-output:KGBone.exe > dist-exe/KGBone.exe
  ```
  (`build-info.txt`의 커밋 해시가 원하는 커밋인지 확인)
- 스마트 앱 컨트롤이 꺼진(또는 없는) PC라면 `npm run build:exe`로 직접 빌드해도 된다. 그 경우 네이티브 `cargo test`도 돈다(`cd src-tauri && cargo test`).
- 서명이 없으므로 처음 실행 때 Windows 보안 경고(SmartScreen)가 뜰 수 있다.

## 4-1. 웹 판(설치형 웹 앱, exe 없이 쓰기)
- 스마트 앱 컨트롤이 켜진 PC는 CI가 만든 exe도 실행이 막힌다. 그런 PC는 웹 판을 쓴다.
- `.github/workflows/deploy-pages.yml`이 main에 push할 때마다 화면을 빌드해 GitHub Pages에 올린다: https://dudwo2199.github.io/KGBone/
  - 처음 한 번은 저장소 Settings → Pages → Source를 **GitHub Actions**로 바꿔야 한다.
- Edge로 위 주소를 열고 주소창 오른쪽의 "앱 설치"(또는 ⋯ → 앱 → 이 사이트를 앱으로 설치)를 누르면 시작 메뉴에 KGBone이 생기고 독립 창으로 열린다.
  - 설치하면 `.kgbone` 더블클릭으로 열 수 있다(처음 열 때 허용 여부를 묻는다).
  - 한 번 열어 두면 인터넷이 없어도 열린다(서비스 워커 `app/public/sw.js`, 목록은 빌드 때 `tools/vite-precache.mjs`가 채운다). 온라인이면 항상 최신 판을 쓴다.
- 화면 빌드는 상대 경로(`base: './'`)라서 Pages(/KGBone/ 아래), 내 PC 서버(`npm run app`), Tauri 모두 같은 빌드로 돈다.

## 4-2. MCP 서버(Claude가 직접 편집하기)
- `tools/mcp/server.mjs`: 창 없이 `kgbone.wasm`을 Node로 돌리는 MCP 서버. 저장소 루트의 `.mcp.json`에 등록돼 있어 이 폴더에서 Claude Code를 열면 승인 후 붙는다.
- 도구: `doc_new`·`doc_open`·`doc_save`(파일), `doc_inspect`(구조 읽기), `command_list`·`command_run`·`command_batch`(`interface/commands.json`의 core 명령), `pose`(본 월드 값), `render`(PNG 그림), `images_reload`, `export_spine`, `export_png_sequence`.
- 파일은 서버를 띄운 폴더 안에서만 읽고 쓴다. 다른 폴더를 더 허용하려면 환경 변수 `KGBONE_MCP_ROOTS`(`;`로 구분).
- **MCP 연결(창 잇기)**: 먼저 뜬 MCP 서버가 `127.0.0.1:47821`에 중계소(WebSocket)를 열고, 창에서 도구 → MCP 연결을 누르면 창이 거기 붙는다. 그러면 도구가 창의 문서를 고친다(화면 명령·`view_screenshot`도 됨, 상태는 `app_status`). 나중에 뜬 다른 MCP 서버(Cursor·Codex 등)는 그 중계소를 거쳐 같은 창에 붙고, 중계소가 꺼지면 남은 서버가 이어받는다. 메시지 형식 `interface/agent-link.md`, 사용 설명서 9장(AI 프로그램별 설정), SPEC N, GAPS G-29. 환경 변수: `KGBONE_MCP_PORT`, `KGBONE_MCP_LIVE=0`(끄기), `KGBONE_MCP_ORIGINS`(허용 웹 주소 추가).
- `app/public/kgbone.wasm`이 필요하다. Rust가 있으면 `npm run wasm`, 없으면 `npm run wasm:fetch`(GitHub Pages에 올라간 origin main 빌드를 받는다 — Rust 핵심을 이 PC에서 고쳤다면 쓰면 안 된다).
- Rust 없이 화면 흐름 테스트를 돌리려면: `npm run wasm:fetch` 후 `npx vite --config app/vite.config.ts`를 띄워 두고 `npx playwright test`.
- 도구 추가(창 연결용): `app_status`(연결 상태), `view_screenshot`(창 뷰포트 PNG). 창이 붙으면 화면 명령(scope: ui)도 `command_run`으로 쓴다.

### 4-3. Claude에서 MCP 쓰는 법 (사용자용 요약)
처음 한 번 등록하고, 쓸 때는 KGBone에서 [연결]을 누르고 말로 시키면 된다. Cursor·Codex·VS Code·Gemini CLI 설정은 사용 설명서 9장.

**한 번만 하는 것 / 매번 하는 것**
| 언제 | 할 일 |
|---|---|
| 처음 한 번 | Node.js 22 이상 설치 → 릴리스 zip 받아 풀기 → AI 프로그램에 등록(아래 "등록") |
| 쓸 때마다 | AI 프로그램 켜기 → KGBone에서 [연결] ("창을 열 때 자동으로 연결"을 켜 두면 이것도 생략) → 말로 시키기 |
| 새 판이 나왔을 때 | 새 zip을 **같은 폴더에 덮어 풀기** → AI 프로그램의 MCP 서버 다시 켜기. 폴더가 같으면 등록은 다시 할 필요 없음 |

압축 풀기와 README의 등록은 **처음 한 번만** 한다. 폴더를 옮기면 등록의 경로만 고친다.

**릴리스 받기(다른 PC, git·npm·Rust 필요 없음)**
1. 브라우저로 https://github.com/dudwo2199/KGBone/releases 를 열고 최신 "KGBone MCP 서버 x.y.z"의 `kgbone-mcp-x.y.z.zip`을 받는다.
   - 명령으로 받기(0.2.0 예):
     ```bash
     curl -L -o kgbone-mcp.zip https://github.com/dudwo2199/KGBone/releases/download/mcp-v0.2.0/kgbone-mcp-0.2.0.zip
     ```
2. 원하는 곳에 푼다(예: `D:/tools/` → `D:/tools/kgbone-mcp/`). 안의 폴더 구조는 바꾸지 않는다(서버가 `interface/`, `app/public/`을 찾음).
3. 안의 `README.md`대로 AI 프로그램에 등록한다. 아래 "등록"의 경로 `D:/workspace/KGSpine/KGBone`을 푼 폴더(`D:/tools/kgbone-mcp`)로 바꾸면 된다.

**릴리스 만들기(개발자)**: `npm run pack:mcp`로 `dist-mcp/kgbone-mcp-<버전>.zip`을 만들어 볼 수 있다. 실제 릴리스는 `mcp-v0.2.1`처럼 버전을 올린 태그를 push하면 `.github/workflows/release-mcp.yml`이 빌드·테스트 후 올린다(첫 릴리스 `mcp-v0.2.0`, 2026-11-17, 라이선스 없이 올림 — 사용자 결정). `tools/mcp/`나 핵심(wasm)을 고쳤으면 새 태그로 다시 올린다.

**등록**
- **Claude Code, KGBone 저장소 폴더에서 열 때**: 할 일 없음. `.mcp.json`으로 자동 등록되고, 처음에 MCP 서버를 쓸지 물으면 허용한다.
- **Claude Code, 다른 폴더에서도 쓸 때**: 한 번만 실행(`-s user` = 모든 폴더, `KGBONE_MCP_ROOTS` = 파일을 읽고 쓸 폴더, 여러 개면 `;`).
  ```bash
  claude mcp add kgbone -s user -e KGBONE_MCP_ROOTS=D:/내작업 -- node D:/workspace/KGSpine/KGBone/tools/mcp/server.mjs
  ```
- **Claude 데스크톱 앱**: 설정 → 개발자 → 설정 편집(`claude_desktop_config.json`)에 넣고 앱을 다시 켠다.
  ```json
  {
    "mcpServers": {
      "kgbone": {
        "command": "node",
        "args": ["D:/workspace/KGSpine/KGBone/tools/mcp/server.mjs"],
        "env": { "KGBONE_MCP_ROOTS": "D:/내작업" }
      }
    }
  }
  ```

**쓸 때마다**
1. Claude를 켠다(kgbone 서버가 같이 켜지고 연결 창구 127.0.0.1:47821을 연다).
2. KGBone에서 도구 → MCP 연결… → [연결]. "창을 열 때 자동으로 연결"을 켜 두면 생략.
3. 말로 시킨다. 예: "팔 본 두 개 만들고 화면 맞춰줘", "걷기 애니메이션 만들어서 0초·0.5초에 다리 회전 키 넣어줘", "지금 화면 찍어서 보여줘", "D:/내작업/char.kgbone 열어줘", "저장해줘".

**주의**
- 필요한 것: Node.js 22 이상, `app/public/kgbone.wasm`(`npm run wasm` 또는 `npm run wasm:fetch`).
- **서버 코드(`tools/mcp/`)를 고친 뒤에는 Claude 쪽 서버를 다시 켜야 반영된다.** Claude Code는 `/mcp`에서 다시 연결하거나 새 세션, 데스크톱 앱은 앱 재시작. 옛 서버가 계속 돌면 창 연결이 안 되거나 새 도구가 안 보인다(도구 수로 확인: 창 연결 판은 14개).
- 연결이 됐는지 모르겠으면 Claude에게 "KGBone 연결 상태 확인해줘"(→ `app_status`).
- 창 상태 "AI 프로그램을 기다리는 중" = 연결 창구를 연 MCP 서버가 없음(Claude가 꺼졌거나 옛 서버). "다른 KGBone 창이 연결돼 있어 기다리는 중" = 다른 창이 연결을 쓰는 중, 이 창으로 가져오려면 [연결 끊기] 뒤 [연결].

## 5. 구조 요약
- `crates/kgbone-core`: 계산 로직 전부(모델, .kgbone/Spine JSON/atlas, 뼈대 계산·제약·물리, 명령, 되돌리기, 내보내기). `api.rs`가 JSON 요청 처리기.
- `crates/kgbone-wasm`: C ABI로 핵심을 화면에 내보냄(wasm-bindgen 없음).
- `src-tauri`: 앱 본체(별도 워크스페이스). 명령 목록은 `interface/tauri-commands.md`.
- `app`: React 화면. 플랫폼 3종(`platform/tauri.ts`, `browser-fs.ts`(File System Access), `web.ts`(메모리, 테스트용)).
- 명령 원본: `interface/commands.json` (메뉴·단축키·버튼 공용, Rust 테스트가 등록 누락을 잡음).

## 6. 9단계에서 할 일 (체크리스트, 끝남)
- [x] `build-output`에서 최신 커밋(9c2e837 이후) 빌드 결과 확인, `KGBone.exe` 받기(`dist-exe/`는 .gitignore에 추가하거나 커밋하지 않기)
- [x] `docs/LOG.md`에 9단계 끝난 시각 기록
- [x] 최종 보고(`docs/REPORT.md`): 걸린 시간(LOG 기준), 파일 수, Rust/TS 줄 수, 테스트 수, 3분 시연 순서, exe 경로, 서명 없음 경고 안내
- [x] 커밋(`[ADD]/[CHG]/[DEL]/[FIX]` + 한국어, 공동 작업자 줄 금지) 후 `git push origin main` (`--force` 금지)

## 7. 참고
- 루트의 `AGENTS.md`는 이 작업에서 만든 파일이 아니다(누가 만들었는지 확인 전이라 커밋하지 않음).
- esotericsoftware.com은 원래 PC에서 접속이 끊겨 검색 요약으로만 근거를 확인했다(GAPS G-01). 접속되는 PC라면 `docs/SOURCES.md`의 URL을 다시 확인할 것.
