# KGBone 규칙집

KGBone은 2D 스켈레탈 애니메이션 에디터다. Windows 11용, 화면은 한국어. 기본 프로젝트 파일은 `.kgbone`(docs/FORMAT.md).

## 1. 클린룸 규칙 (가장 중요)
- 원본 프로그램(Esoteric Software의 에디터)의 코드는 보지도, 디컴파일하지도 않는다. 같은 기능의 오픈소스 프로그램 코드도 복사하지 않는다.
- `EsotericSoftware/spine-runtimes` 저장소는 코드·문서·CHANGELOG를 포함해 **어떤 파일도 열지 않는다**. 공식 런타임(spine-ts, spine-player, spine-webcomponents 등)은 구현에도, 테스트 정답 비교에도 쓰지 않는다.
- 근거는 ① esotericsoftware.com의 사용 설명서·블로그·변경 이력 ② 공개 파일 형식 문서(JSON, 바이너리, 아틀라스) ③ 직접 만든 시험 파일만. 참고한 문서는 URL·확인 날짜·용도를 `docs/SOURCES.md`에 적는다.
- 공식 예제 파일(spineboy 등)과 그 이미지는 쓰지 않는다. 시험 파일·이미지는 `tools/gen-samples.mjs`로 직접 만든다.
- 원본의 이름·로고·아이콘·폰트·템플릿은 쓰지 않는다. 앱 이름·창 제목·아이콘·메뉴에 "Spine"을 쓰지 않는다(가져오기·내보내기의 "Spine JSON 호환" 같은 형식 설명만 허용).
- 범용 라이브러리(압축, 이미지, JSON, 화면 프레임워크, 2D 렌더링)는 써도 된다. 단 `kgbone-core`는 아래 4장 사정으로 외부 의존성 0개를 유지한다.

## 2. 폴더 구조와 모듈 역할
```
crates/kgbone-core/   Rust 핵심(계산 로직은 여기에만). 앱 본체와 WebAssembly가 함께 쓴다.
  src/model/          공통 데이터 모델(FORMAT.md와 1:1)
  src/json/           키 순서·숫자 표기 보존 JSON(parse, merge, write), ToJson/FromJson, json_object! 매크로
  src/format/         파일 형식: kgbone.rs, spine_json/(read, write), atlas/(read, write), migrate.rs — SkeletonFormat trait
  src/pose/           뼈대 계산: bone_transform, ik, transform, path, physics, slider, mesh_deform, clipping — ConstraintSolver trait
  src/anim/           곡선 보간(curve.rs), 애니메이션 적용(apply.rs)
  src/mesh/           삼각분할, 알파로 메시 자동 생성, 가중치(바인딩·자동·페인팅)
  src/commands/       명령(기능별 파일: bone.rs, slot.rs …) — Command trait, interface/commands.json과 1:1
  src/history.rs      되돌리기 기록 — UndoEntry trait
  src/document.rs     열린 문서(모델 + 기록 + 원본 보존). 바깥은 execute로만 편집
  src/api.rs          JSON 요청 처리기(화면·앱 본체 공용)
  src/export/         텍스처 패킹, PNG 읽기·쓰기, 소프트웨어 그리기, 내보내기 묶음
  src/expr.rs         숫자 입력칸 수식
crates/kgbone-wasm/   핵심을 WebAssembly C ABI로 내보내는 얇은 층
src-tauri/            앱 본체(Tauri 2): 파일·창·레지스트리. 계산은 kgbone-core에 맡김
app/                  화면(React + TypeScript + Vite)
  src/core/           types.ts(모델 타입), bridge.ts(Core 인터페이스), wasm.ts(불러오기)
  src/platform/       Platform 인터페이스, tauri.ts, web.ts(메모리 파일, 자동 테스트용)
  src/commands/       화면 명령 등록: 메뉴·단축키·버튼이 모두 같은 명령 id를 부른다
  src/render/         WebGL2 미리보기(첨부물 종류별 그리기)
  src/ui/             패널: 메뉴, 도구 모음, 뷰포트, 트리, 속성, 도프시트, 그래프, 재생
interface/            commands.json(명령 목록 원본), core-api.md, tauri-commands.md
docs/                 SPEC, UI_SPEC, FORMAT, SOURCES, GAPS, LOG
samples/              직접 만든 시험 파일(tools/gen-samples.mjs)
e2e/                  Playwright 화면 흐름 테스트
tools/                생성기·빌드·테스트 실행 도우미
```

## 3. 코드 구성
- 되도록 기능별로 파일을 나눈다(예: 본 생성, IK 계산, atlas 읽기를 각각 다른 파일로). 한 파일이 여러 기능을 떠안지 않게 한다.
- 캡슐화: 모듈 내부 데이터는 감추고, 밖에서는 정해진 함수·명령으로만 바꾸게 한다(Rust는 pub 범위 최소화, TypeScript는 export 최소화).
- 추상화: 공통 동작은 trait/interface로 묶는다(예: 제약 종류별 계산 `ConstraintSolver`, 첨부물 종류별 그리기, 파일 형식별 읽기·쓰기 `SkeletonFormat`, 명령 실행·되돌리기 `Command`/`UndoEntry`).
- 나누면 오히려 억지스러운 경우(하나의 알고리즘, 서로 강하게 얽힌 작은 코드)는 한 파일에 둬도 된다.
- 확정한 인터페이스(`interface/`, `app/src/core/*.ts`, `app/src/platform/platform.ts`, 모델 타입, `api.rs` 메서드)는 함부로 바꾸지 않는다. 바꿔야 하면 양쪽을 같이 고치고 문서도 고친다.
- 모든 편집은 되돌리기(Ctrl+Z)·다시 실행(Ctrl+Y, Ctrl+Shift+Z)이 된다. 드래그 한 번은 `history.begin`~`history.end`로 묶어 되돌리기 한 번.
- 글자 입력칸은 한국어 IME 조합 중(`isComposing`)에 단축키를 처리하지 않는다.
- 손대지 않은 데이터는 저장해도 그대로: .kgbone 무편집 저장 = 바이트 동일, 편집 저장 = 고친 부분만 바뀜, Spine JSON 내보내기 = 모르는 필드·키 순서·숫자 표기 보존(`json::merge`).

## 4. 이 PC의 제약 (스마트 앱 컨트롤)
- Windows 스마트 앱 컨트롤이 새로 빌드한 서명 없는 .exe(Rust 테스트 실행 파일, 빌드 스크립트)를 막는다(os error 4551). 보안 설정은 바꾸지 않는다.
- 그래서 `kgbone-core`·`kgbone-wasm`은 **외부 크레이트 0개**(빌드 스크립트·proc-macro 없음). 직렬화는 `json_object!` 매크로로 한다.
- Rust 테스트는 `wasm32-wasip1`로 빌드해 Node WASI로 돌린다(`.cargo/config.toml`의 runner = `tools/wasi-run.mjs`). panic=abort이므로 실패한 테스트에서 실행이 멈춘다.
- Tauri 앱은 이 PC에서 빌드할 수 없으므로 GitHub Actions(`.github/workflows/build-windows.yml`)에서 빌드한다.
- CI가 만든 exe도 서명이 없어 이 PC에서는 실행이 막힌다. 이 PC에서는 웹 판(GitHub Pages에 올린 설치형 웹 앱, 또는 `npm run app`)으로 쓴다.

## 5. 빌드·실행·테스트 명령
```bash
npm install                      # 화면 의존성
npm run samples                  # 시험 파일 다시 만들기
npm run wasm                     # Rust 핵심 → app/public/kgbone.wasm
npm run dev                      # 브라우저 개발 서버 http://localhost:1420 (WebPlatform)
npm run build                    # 화면 배포 빌드(app/dist)
npm run test:rust                # Rust 테스트(WASI) = cargo test -p kgbone-core --target wasm32-wasip1
npm run test:ui                  # 화면 단위 테스트(vitest)
npm run test:e2e                 # 화면 흐름 테스트(Playwright, 설치된 Edge 사용)
npm test                         # 전부
npm run icons                    # 아이콘 다시 만들기(tools/icon.png, app/public/icons)
npx tauri build                  # 실행 파일(이 PC에서는 막힘 → GitHub Actions)
```
웹 판(설치형 웹 앱)은 main에 push하면 `.github/workflows/deploy-pages.yml`이 GitHub Pages에 올린다. 화면 빌드는 상대 경로(`base: './'`)를 쓰므로 화면 코드에서 `/`로 시작하는 절대 경로로 파일을 부르지 않는다.
PATH에 cargo가 없으면 `%USERPROFILE%\.cargo\bin`을 앞에 붙인다.

## 6. '완료'의 정의
- docs/SPEC.md의 해당 기능 '완료 기준'을 확인하는 자동 테스트가 있고 통과한다(테스트 이름에 기능 번호, 예: `f_b01_...`).
- `npm run test:rust`, `npm run test:ui`, `npm run test:e2e`가 모두 통과한다. 같은 실패를 3번 고쳐도 안 되면 원인과 함께 docs/GAPS.md에 적는다.
- 화면 글자는 한국어이고, 메뉴·단축키·버튼은 `interface/commands.json`의 같은 명령 id를 부른다.
- 아직 안 되는 것은 docs/GAPS.md에 적혀 있다.

## 7. 커밋 규칙
- 커밋 메시지는 반드시 아래 접두사로 시작하고, 뒤에 무엇을 했는지 한국어로 짧게 적는다.
  - `[ADD]` 파일 추가
  - `[CHG]` 파일 변경
  - `[DEL]` 파일 삭제
  - `[FIX]` 버그 수정
  - 예: `[ADD] 본 생성 명령과 테스트 추가` / `[FIX] IK 대상이 부모 본일 때 회전이 뒤집히는 문제 수정`
- 단계가 끝날 때마다 `origin main`에 push한다. 강제 push(`--force`)는 하지 않는다.
- 커밋 메시지에 Co-Authored-By 같은 공동 작업자 줄(Claude/Anthropic 포함)을 넣지 않는다. PR 설명에도 생성 도구 표시를 넣지 않는다.
- 한 커밋에는 되도록 한 종류만 담는다. 섞이면 가장 중요한 변경의 접두사를 쓰고, 버그 수정이 들어 있으면 `[FIX]`를 우선한다.
- `.gitignore`의 `.claude/` 항목은 지우지 않는다.
