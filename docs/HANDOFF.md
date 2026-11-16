# 이어서 작업하기 (다른 PC용 인수인계)

마지막 갱신: 2026-11-17. 작업 명세 원본은 저장소 밖의 `KGBone 제작 프롬프트.txt`(0~9단계)이고, 규칙은 `CLAUDE.md`에 모두 옮겨 두었다.

## 1. 지금 어디까지 했나
| 단계 | 상태 |
|---|---|
| 0~8단계 | 끝남 (docs/LOG.md에 단계별 끝난 시각) |
| 9단계 마무리 | **남음**: 최신 커밋의 실행 파일 받기, LOG 기록, 최종 보고(시간·파일 수·코드 줄 수·테스트 수 표, 3분 시연 순서, exe 경로) |
| 9단계 뒤 추가 | 웹 판(설치형 웹 앱, GitHub Pages), MCP 서버, 뷰포트 그림 끌어 옮기기(D14), README — 끝남 |
| 유니티 런타임 1단계 | 끝남(배포 전, 아래 1-1) |

- 테스트: Rust 165개(핵심 156 + 앱 본체 9, 앱 본체는 CI에서만 실행), 화면 단위 56개, MCP 서버 12개, 화면 흐름(Playwright) 6개 — 모두 통과.
- 코드: Rust 약 17,600줄 / TypeScript 약 8,700줄, 저장소 파일 약 270개.
- 아직 안 되는 것·미확인 표기: `docs/GAPS.md`.

### 1-1. 유니티 런타임 1단계 (끝남, 배포 전)
- 설계 `docs/superpowers/specs/2026-11-16-unity-runtime-stage1-design.md`, 계획 `docs/superpowers/plans/2026-11-16-unity-runtime-stage1.md`, 명세 SPEC.md M절.
- 패키지 `unity/com.kgbone.runtime/`: 계산부(C#)가 KGBone 핵심과 같은지 정답 64개로 비교(`npm run unity:test`, CI 워크플로 `unity-core.yml`). 유니티 EditMode 104개(`npm run unity:editmode`).
- 확인용 프로젝트 `unity/KGBoneSandbox/`: `npm run unity:sandbox -- <폴더>`로 파일을 넣고, 메뉴 KGBone > 화면 찍기(또는 배치 모드 `-executeMethod KGBoneSandbox.SandboxTools.CaptureBatch`)로 `Logs/capture-*.png`를 만든다.
- 이 PC: Unity 6000.3.17f1, .NET SDK 10. `dotnet test`는 스마트 앱 컨트롤에 막히지 않는다.
- 다음 단계 후보: 2단계(제약), 3단계(클리핑·시퀀스·섞기·스킨 바꾸기·본 덮어쓰기). 미확인 사항은 GAPS G-26~G-28.
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
- `app/public/kgbone.wasm`이 필요하다. Rust가 있으면 `npm run wasm`, 없으면 `npm run wasm:fetch`(GitHub Pages에 올라간 origin main 빌드를 받는다 — Rust 핵심을 이 PC에서 고쳤다면 쓰면 안 된다).
- Rust 없이 화면 흐름 테스트를 돌리려면: `npm run wasm:fetch` 후 `npx vite --config app/vite.config.ts`를 띄워 두고 `npx playwright test`.

## 5. 구조 요약
- `crates/kgbone-core`: 계산 로직 전부(모델, .kgbone/Spine JSON/atlas, 뼈대 계산·제약·물리, 명령, 되돌리기, 내보내기). `api.rs`가 JSON 요청 처리기.
- `crates/kgbone-wasm`: C ABI로 핵심을 화면에 내보냄(wasm-bindgen 없음).
- `src-tauri`: 앱 본체(별도 워크스페이스). 명령 목록은 `interface/tauri-commands.md`.
- `app`: React 화면. 플랫폼 3종(`platform/tauri.ts`, `browser-fs.ts`(File System Access), `web.ts`(메모리, 테스트용)).
- 명령 원본: `interface/commands.json` (메뉴·단축키·버튼 공용, Rust 테스트가 등록 누락을 잡음).

## 6. 9단계에서 할 일 (체크리스트)
- [ ] `build-output`에서 최신 커밋(9c2e837 이후) 빌드 결과 확인, `KGBone.exe` 받기(`dist-exe/`는 .gitignore에 추가하거나 커밋하지 않기)
- [ ] `docs/LOG.md`에 9단계 끝난 시각 기록
- [ ] 최종 보고: 걸린 시간(LOG 기준), 파일 수, Rust/TS 줄 수, 테스트 수, 3분 시연 순서, exe 경로, 서명 없음 경고 안내
- [ ] 커밋(`[ADD]/[CHG]/[DEL]/[FIX]` + 한국어, 공동 작업자 줄 금지) 후 `git push origin main` (`--force` 금지)

## 7. 참고
- 루트의 `AGENTS.md`는 이 작업에서 만든 파일이 아니다(누가 만들었는지 확인 전이라 커밋하지 않음).
- esotericsoftware.com은 원래 PC에서 접속이 끊겨 검색 요약으로만 근거를 확인했다(GAPS G-01). 접속되는 PC라면 `docs/SOURCES.md`의 URL을 다시 확인할 것.
