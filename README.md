# KGBone

2D 스켈레탈 애니메이션 에디터. 그림 파츠를 본(뼈)에 붙이고, 본을 움직여 애니메이션을 만든다.
Windows 11용이고 화면은 한국어다.

**바로 써 보기: https://dudwo2199.github.io/KGBone/** (Edge 또는 Chrome)

## 주요 기능

- **본**: 계층, 이동·회전·크기·기울기, 상속 5종, 한글 이름
- **슬롯·첨부물**: 이미지, 메시(정점 편집·자동 생성·가중치 바인딩·자동 가중치·가중치 붓), 링크드 메시, 바운딩 박스, 포인트, 클리핑, 패스, 시퀀스
- **제약**: IK, 트랜스폼, 패스, 물리, 슬라이더
- **스킨·이벤트**
- **애니메이션**: 셋업/애니메이트 모드, 도프시트, 그래프(베지어 곡선), 오니언 스킨, 재생·반복
- **파일**: `.kgbone` 프로젝트(무편집 저장 시 바이트 동일), Spine JSON 호환 가져오기·내보내기(4.2/4.3, 아틀라스·텍스처 패킹 포함), PNG 시퀀스 내보내기
- **되돌리기**: 모든 편집을 Ctrl+Z / Ctrl+Y로. 드래그 한 번은 되돌리기 한 번
- **숫자 입력칸 수식**: `+=5`, `v*2`, `(1+2)*3`

전체 기능과 완료 기준은 [docs/SPEC.md](docs/SPEC.md), 아직 안 되는 것은 [docs/GAPS.md](docs/GAPS.md)에 있다.

## 설치와 실행

세 가지 판이 있고, 화면과 계산 코드는 모두 같다.

| 판 | 실행 방법 | 비고 |
|---|---|---|
| **웹 판 (권장)** | 위 주소를 Edge로 열고 주소창 오른쪽 **"앱 설치"** | 시작 메뉴에 생기고 독립 창으로 열린다. `.kgbone` 더블클릭 열기, 오프라인 실행 지원 |
| 내 PC 서버 판 | `npm run app` 또는 `KGBone 실행.cmd` | Node.js와 Edge 필요. 내 PC 안(127.0.0.1)에서만 서비스 |
| 실행 파일 (.exe) | GitHub Actions가 빌드해 `build-output` 브랜치에 올린다 | 코드 서명이 없어 SmartScreen 경고가 뜨고, 스마트 앱 컨트롤이 켜진 PC에서는 실행이 막힌다 |

웹 판과 내 PC 서버 판은 브라우저의 File System Access로 내 PC의 실제 파일을 열고 저장한다.
프로젝트를 열 때 폴더를 고르라고 하면 **프로젝트 파일이 있는 폴더**를 고른다. 그래야 옆의 이미지 폴더까지 읽힌다.

## 처음 써 보기

1. **파일 → 이미지 폴더 지정**으로 파츠 PNG가 든 폴더를 고른다(기본은 프로젝트 옆 `images/`).
2. **생성 도구(N)**로 뷰포트를 끌어 본을 만든다. 본을 고른 채 끌면 그 본의 자식이 된다.
3. **만들기 → 슬롯**, **만들기 → 첨부물 → 이미지**로 본에 그림을 붙인다.
4. **만들기 → 애니메이션**으로 애니메이션을 만들고, **Tab**으로 애니메이트 모드로 바꾼다. 시점을 옮겨 가며 본을 움직이면 키가 생긴다.
5. **D**로 재생하고, **Ctrl+S**로 저장한다.

### 자주 쓰는 단축키

| 키 | 동작 | 키 | 동작 |
|---|---|---|---|
| V / C / X / Z | 이동 / 회전 / 크기 / 기울기 도구 | Tab | 셋업 ↔ 애니메이트 모드 |
| N | 본 생성 도구 | D / A | 재생 / 거꾸로 재생 |
| B | 포즈 도구 | , / . | 이전 / 다음 프레임 |
| G | 가중치 붓 | K | 바뀐 값 키 지정 |
| + / - | 그리기 순서 앞으로 / 뒤로 | O | 오니언 스킨 |
| F2 | 이름 바꾸기 | Home | 화면 맞춤 |
| Ctrl+Z / Ctrl+Y | 되돌리기 / 다시 실행 | F1 | 단축키 목록 |

- 휠: 확대·축소. 가운데 버튼 드래그 또는 Space+드래그: 화면 이동. 오른쪽 클릭: 직전 도구로 바꾸기.
- 셋업 모드에서 이동 도구로 **그림**을 끌면 그 첨부물만 옮겨지고, **본**을 끌면 본째로 옮겨진다.
- 전체 메뉴·단축키 표는 [docs/UI_SPEC.md](docs/UI_SPEC.md)에 있다.

## 파일 형식

| 형식 | 용도 |
|---|---|
| `.kgbone` | KGBone 프로젝트(JSON). 형식 문서: [docs/FORMAT.md](docs/FORMAT.md) |
| Spine JSON 호환 `.json` + `.atlas` + `.png` | 다른 도구와 주고받기. 모르는 필드·키 순서·숫자 표기를 보존한다 |
| PNG 시퀀스 | 애니메이션을 프레임 그림으로. 게임 엔진의 스프라이트 애니메이션으로 바로 쓸 수 있다 |

## 유니티에서 재생하기

`unity/com.kgbone.runtime`은 `.kgbone`을 유니티(Unity 6 이상, URP)에서 KGBone과 같은 결과로 재생하는 패키지다. 다른 런타임 없이 이것만으로 재생한다.

1. Package Manager → **Add package from git URL** → `https://github.com/dudwo2199/KGBone.git?path=/unity/com.kgbone.runtime#main`
2. `.kgbone` 파일과 그 이미지 폴더를 `Assets` 아래에 넣으면 데이터 에셋·텍스처·머티리얼이 자동으로 생긴다.
3. 빈 GameObject에 **KGBoneAnimation**을 붙이고 데이터와 애니메이션을 고른다.

지금은 1단계(재생 + 본 위치 읽기)다. 제약·클리핑·시퀀스·애니메이션 섞기는 아직 없다. 자세한 사용법: [unity/com.kgbone.runtime/README.md](unity/com.kgbone.runtime/README.md)
아직 버전 태그와 라이선스가 없다(배포 전).

## 개발

### 준비

- Node.js 24
- Rust(rustup)와 타깃: `rustup target add wasm32-unknown-unknown wasm32-wasip1`
- 실행 파일까지 직접 빌드하려면 Visual Studio C++ 빌드 도구

```bash
npm install
npm run wasm        # Rust 핵심 → app/public/kgbone.wasm (Rust가 없으면 npm run wasm:fetch)
npm run dev         # 개발 서버 http://localhost:1420
```

### 명령

| 명령 | 하는 일 |
|---|---|
| `npm run build` | 화면 배포 빌드(`app/dist`) |
| `npm run app` | 배포 빌드 후 Edge 앱 창으로 열기 |
| `npm run build:exe` | Tauri 실행 파일 |
| `npm run samples` | 시험 파일 다시 만들기 |
| `npm run icons` | 앱 아이콘 다시 만들기 |
| `npm test` | 아래 테스트 전부 |
| `npm run test:rust` | Rust 핵심 테스트(wasm32-wasip1로 빌드해 Node로 실행) |
| `npm run test:ui` | 화면 단위 테스트(vitest) |
| `npm run test:mcp` | MCP 서버 테스트 |
| `npm run test:e2e` | 화면 흐름 테스트(Playwright, 설치된 Edge 사용) |
| `npm run unity:test` | 유니티 런타임 계산부 테스트(dotnet test, KGBone 핵심과 정답 비교) |
| `npm run unity:golden` | 유니티 런타임 정답 파일 다시 만들기 |
| `npm run unity:editmode` | 유니티 EditMode 테스트(Unity 6000.3 필요) |

main에 push하면 GitHub Actions가 웹 판을 GitHub Pages에 배포하고(`deploy-pages.yml`), Windows 실행 파일을 빌드하고(`build-windows.yml`), 유니티 런타임 계산부를 정답과 비교한다(`unity-core.yml`).

### 구조

```
crates/kgbone-core/   Rust 핵심: 모델, 파일 형식, 뼈대·제약·물리 계산, 명령, 되돌리기, 내보내기 (외부 크레이트 0개)
crates/kgbone-wasm/   핵심을 WebAssembly(C ABI)로 내보내는 얇은 층
src-tauri/            실행 파일 판 앱 본체(Tauri 2)
app/                  화면(React + TypeScript + Vite, WebGL2 미리보기)
interface/            명령 목록(commands.json)과 연결 인터페이스 문서
unity/                유니티 런타임 패키지(com.kgbone.runtime), 계산부 테스트, 확인용 유니티 프로젝트
tools/                생성기, 빌드 도우미, MCP 서버(tools/mcp)
samples/              직접 만든 시험 파일
e2e/                  화면 흐름 테스트
docs/                 명세(SPEC, UI_SPEC, FORMAT), 근거(SOURCES), 미구현(GAPS), 기록(LOG), 인수인계(HANDOFF)
```

계산 로직은 `kgbone-core` 한 곳에만 있고, 화면은 이것을 WebAssembly로, 실행 파일 판은 Rust 라이브러리로 쓴다. 유니티 런타임만 예외로 C#으로 다시 구현하고, 핵심이 만든 정답과 자동으로 비교한다.
메뉴·단축키·버튼은 모두 [interface/commands.json](interface/commands.json)의 같은 명령 id를 부른다.
작업 규칙은 [CLAUDE.md](CLAUDE.md)에 있다.

### MCP 서버 (AI 에이전트로 편집하기)

`tools/mcp/server.mjs`는 창 없이 `kgbone.wasm`을 Node로 돌리는 [MCP](https://modelcontextprotocol.io) 서버다.
저장소 루트의 `.mcp.json`에 등록돼 있어, 이 폴더에서 Claude Code를 열면 쓸 수 있다.

- 도구: 문서 열기·저장, 구조 읽기, 편집 명령 실행(묶음 = 되돌리기 한 번), 본 계산 결과 읽기, PNG로 그려 보기, 내보내기
- 파일은 서버를 띄운 폴더 안에서만 읽고 쓴다(더 허용하려면 환경 변수 `KGBONE_MCP_ROOTS`)
- `app/public/kgbone.wasm`이 필요하다(`npm run wasm` 또는 `npm run wasm:fetch`)

## 클린룸 원칙

KGBone은 공개된 사용 설명서와 파일 형식 문서, 직접 만든 시험 파일만을 근거로 처음부터 작성했다.
원본 에디터나 공식 런타임의 코드는 보지 않았고, 공식 예제 파일·이미지도 쓰지 않았다. 참고한 문서는 [docs/SOURCES.md](docs/SOURCES.md)에 있다.

"Spine"은 Esoteric Software의 상표다. KGBone은 Esoteric Software와 관계가 없으며, "Spine JSON 호환"은 파일 형식을 설명하는 말로만 쓴다.
