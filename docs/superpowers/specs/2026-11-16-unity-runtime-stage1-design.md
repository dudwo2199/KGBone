# KGBone 유니티 런타임 1단계 설계

작성: 2026-11-16. 상태: 승인 대기.

## 1. 목적과 이유

KGBone에서 만든 뼈대 애니메이션을 **유니티 게임에서 그대로** 재생한다.
"Spine JSON 호환" 내보내기를 공식 Spine 런타임으로 재생하면 그 런타임의 라이선스 조건(Spine 에디터 라이선스 필요)이 걸리므로, KGBone 전용 런타임을 직접 만든다.
결과는 **KGBone 에디터와 같아야** 한다. 에디터에서 본 움직임이 게임에서 달라지면 이 런타임을 쓸 이유가 없다.

전체는 3단계로 나눈다. 이 문서는 1단계만 다룬다.

| 단계 | 내용 |
|---|---|
| **1 (이 문서)** | 기본 재생 + 본 위치 읽기 |
| 2 | 제약(IK, 트랜스폼, 패스, 물리, 슬라이더) |
| 3 | 클리핑, 시퀀스, 애니메이션 섞기(전환), 실행 중 스킨 바꾸기, 코드로 본 덮어쓰기 |

## 2. 목표 · 비목표 · 반목표

### 목표 (1단계 완료 조건)
1. 저장소의 `unity/com.kgbone.runtime/`에 UPM 패키지가 있고, git 주소(`…KGBone.git?path=/unity/com.kgbone.runtime#unity-v0.1.0`)로 설치된다.
2. `.kgbone` 파일과 그 이미지 폴더를 유니티 `Assets`에 넣으면 가져오기 도구가 자동으로 에셋을 만든다(사람이 따로 만드는 에셋 없음).
3. 컴포넌트를 붙이고 애니메이션을 고르면 재생된다. 코드로 `Play`, 속도, 반복, 이벤트 받기, 좌우·상하 뒤집기, Sorting Layer·순서, 본 위치 읽기가 된다.
4. 지원 기능의 계산 결과가 KGBone 핵심(Rust)과 같다는 것을 자동 비교 테스트로 확인한다(오차 1e-3, 1단계 지원 기능만 쓰는 시험 파일 기준).
5. 골렘 idle 같은 파일(본, 이미지, 키 곡선)이 Unity 6 + URP에서 에디터와 같게 재생된다.

### 1단계 지원 기능
- 본: 계층, 이동·회전·크기·기울기, 상속 5종(`normal`, `onlyTranslation`, `noRotationOrReflection`, `noScale`, `noScaleOrReflection`), 스킨 전용 본(`skinRequired`)의 활성 판정
- 슬롯: 색, 어두운 색(tint black), 셋업 첨부물, 블렌드 모드 4종(normal, additive, multiply, screen), 그리기 순서
- 첨부물: 이미지(region), 메시(비가중치·가중치), 링크드 메시
- 스킨: 기본 스킨 + 시작할 때 고른 스킨 하나(찾는 순서는 핵심과 같게: 고른 스킨 → 기본 스킨)
- 애니메이션 타임라인: 본(rotate, translate·scale·shear와 축별 x/y, inherit), 슬롯(attachment, rgba·rgb·alpha, rgba2·rgb2), deform, drawOrder, events
- 곡선: 선형, 계단, 베지어(값 채널마다 `[cx1, cy1, cx2, cy2]` 절대 좌표, deform은 첫 채널을 0~1 비율 곡선으로)
- 반복 재생 시점 계산과 이벤트 발생 구간 규칙은 핵심(`pose/mod.rs`의 `animation_time`, `pose/events.rs`)과 같게

### 비목표 (1단계에서 하지 않음)
- 제약 5종 계산(2단계). 제약이 있는 파일은 **제약 없이** 재생하고, 가져올 때 경고한다.
- 클리핑, 시퀀스(3단계). 클리핑은 자르지 않고 그리며 경고한다. 시퀀스는 셋업 프레임 이미지로 그리며 경고한다.
- 애니메이션 섞기·전환, 실행 중 스킨 바꾸기, 코드로 본 덮어쓰기, 여러 트랙 동시 재생(3단계)
- 유니티 Animator·Timeline 연동
- `.unitypackage`, 에셋 스토어, OpenUPM 배포
- 유니티에서 Spine JSON 호환 파일 읽기(`.kgbone`만 읽는다)
- Built-in 렌더 파이프라인, Unity 2022 이하

### 반목표 (이렇게 되면 기술적으로 동작해도 실패)
- 에디터와 결과가 다른데 아무 경고가 없는 것. 지원하지 않는 기능은 반드시 가져올 때 경고한다.
- 편집 상태(`editor` 블록) 같은 에디터 전용 데이터가 게임 빌드에 들어가는 것.
- 공식 Spine 런타임 코드를 참고하거나 비교에 쓰는 것(클린룸 규칙, CLAUDE.md 1장).
- 테스트를 돌리려면 유니티 라이선스가 있어야 하는 구조. 계산 비교 테스트는 유니티 없이 돌아야 한다.

## 3. 제약 조건

- **클린룸**: CLAUDE.md 1장 그대로. 근거는 `docs/FORMAT.md`와 KGBone 핵심 코드뿐이다. 패키지·클래스·메뉴 이름에 "Spine"을 쓰지 않는다.
- **규칙 예외**: CLAUDE.md의 "계산 로직은 kgbone-core에만"에 예외를 둔다. 유니티 런타임은 C#으로 계산을 다시 구현하되, KGBone 핵심의 결과와 자동 비교 테스트로 같음을 보장한다. 이 예외를 CLAUDE.md에 적는다(사용자 결정).
- **유니티 환경**: Unity 6(6000.0) 이상, URP(사용자 결정). 이 PC에는 6000.3.17f1이 설치돼 있다.
- **이 PC의 스마트 앱 컨트롤**: 서명 없는 실행 파일이 막힌다. `dotnet test`가 막히는지는 아직 모른다. 막히면 같은 비교 테스트를 유니티 EditMode 테스트로 이 PC에서 돌리고, `dotnet test`는 GitHub Actions에서만 돈다.
- **CI**: GitHub Actions에서 유니티를 돌리려면 유니티 라이선스를 비밀값으로 등록해야 하므로, CI에서는 유니티를 쓰지 않는다. 계산 비교 테스트(`dotnet test`)와 정답 파일 최신 여부 확인만 한다.
- **시험 파일**: 유료 에셋(골렘 미리보기)은 저장소에 넣지 않는다. 시험 파일은 `tools/gen-samples.mjs`로 만든 것만 쓴다.

## 4. 만드는 방식 (검토한 방법)

| 방법 | 내용 | 판단 |
|---|---|---|
| **A. 자체 런타임 + 캐릭터당 메시 하나** | 계산을 C#으로 옮기고, 매 프레임 그리기 목록으로 유니티 Mesh 하나를 다시 채운다 | **채택(사용자 선택).** 결과를 핵심과 숫자로 비교할 수 있고, 캐릭터 하나가 거의 그리기 호출 하나라 빠르며, 2·3단계를 이어 붙이기 쉽다. Animator와는 연결되지 않는다 |
| B. 유니티 2D Animation 패키지로 변환 | 가져올 때 유니티 본·스프라이트 스킨·애니메이션 클립으로 바꾼다 | 제외. 상속 5종, 채널별 절대 좌표 베지어, 그리기 순서 키, 기울기를 근사해야 해서 "에디터와 같은 결과"를 보장할 수 없고, 2단계 제약을 붙이기 어렵다 |
| C. 슬롯마다 SpriteRenderer, 본마다 Transform | 유니티 기본 부품만으로 조립 | 제외. 메시 변형·가중치 메시를 표현할 수 없고, 상속 종류와 기울기가 Transform으로 정확히 안 나온다 |

계산을 옮기지 않는 방법(Rust 핵심을 네이티브 DLL로 부르기, 유니티 안에서 wasm 돌리기)도 검토했다. DLL은 이 PC의 스마트 앱 컨트롤에 막힐 가능성이 높고 플랫폼마다 따로 빌드해야 하며, wasm은 별도 실행 라이브러리가 필요하고 느려서 제외했다(사용자 결정).

## 5. 설계

### 5.1 폴더 구조

```
unity/
  com.kgbone.runtime/            UPM 패키지 (package.json: name "com.kgbone.runtime", unity "6000.0", version "0.1.0")
    Runtime/
      Core/                      순수 C# 계산부. asmdef "KGBone.Runtime.Core", noEngineReferences = true
      (그 밖)                    유니티부. asmdef "KGBone.Runtime" (Core 참조)
    Editor/                      가져오기 도구. asmdef "KGBone.Editor" (Editor 플랫폼만)
    Shaders/                     URP 셰이더
    Tests/
      Golden/                    정답 파일(아래 5.6), 생성기가 만든다
      Editor/                    유니티 EditMode 테스트 (가져오기·컴포넌트·정답 비교)
    README.md, CHANGELOG.md, LICENSE.md(저장소 라이선스가 정해지면)
  tests/KGBone.Core.Tests/       .NET 테스트 프로젝트. Runtime/Core 소스를 링크해 컴파일, 정답 비교
  KGBoneSandbox/                 확인용 유니티 프로젝트. Packages/manifest.json에서 패키지를 file: 경로로 참조
```

`unity/KGBoneSandbox/`의 `Library/`, `Temp/`, `Logs/`, `UserSettings/`, `obj/`와 `unity/tests/**/bin|obj`는 `.gitignore`에 추가한다(`.claude/` 항목은 건드리지 않는다).

### 5.2 계산부 (`Runtime/Core`, 유니티 없음)

KGBone 핵심의 해당 모듈을 같은 순서·같은 규칙으로 옮긴다. 근거 파일은 괄호 안.

1. **JSON 읽기**: 작은 자체 파서. 외부 패키지에 기대지 않는다(Newtonsoft 등 의존성 없음).
2. **모델**: `docs/FORMAT.md`의 skeleton 부분(본, 슬롯, 스킨, 첨부물, 이벤트, 애니메이션). 생략된 필드는 FORMAT.md 기본값. 모르는 필드는 무시한다(런타임은 다시 쓰지 않으므로 보존할 필요 없음). formatVersion 0 변환(FORMAT.md 7장)은 제약 배열에만 영향을 주므로 1단계에서는 무시해도 결과가 같다.
3. **포즈 상태**: 본 로컬 값·상속 종류, 슬롯 색·어두운 색·첨부물, 그리기 순서, deform (`pose/mod.rs`의 `SkeletonPose`에서 제약·시퀀스 부분 뺀 것).
4. **애니메이션 적용**: 섞기 비율 1만(섞기는 3단계). 본 값은 셋업 대비 상대값(rotate·translate·shear 더하기, scale 곱하기), 슬롯 색은 절대값, 이산 값은 계단 (`anim/apply.rs`, `bone_keys.rs`, `slot_keys.rs`, `attachment_keys.rs`, `curve.rs`, 그리기 순서 키).
5. **활성 판정**: `skinRequired` 본은 고른 스킨에 들어 있을 때만 (`pose/active.rs`).
6. **월드 변환**: 상속 5종 (`pose/bone_transform.rs`).
7. **그리기 목록**: 그리기 순서대로 region 네 모서리(왼아래, 오른아래, 오른위, 왼위, uv (0,1)(1,1)(1,0)(0,0), 삼각형 [0,1,2, 2,3,0]), 메시·링크드 메시 정점(가중치·deform 포함), 색 = 슬롯 색 × 첨부물 색, 비활성 본의 슬롯은 그리지 않음 (`pose/draw.rs`, `pose/mesh_deform.rs`).
8. **이벤트**: 구간 [last, time), 반복 재생이면 바퀴마다 (`pose/events.rs`).

계산부의 출력 모양은 핵심의 `PoseOutput`(bones, active, draw, events)과 같은 정보를 담아서, 정답 파일과 항목별로 비교할 수 있게 한다. 숫자는 핵심과 같은 32비트 `float`으로 계산한다.

### 5.3 데이터 에셋과 가져오기 도구 (`Editor/`)

- `ScriptedImporter`가 확장자 `.kgbone`을 맡는다. 가져오면 메인 에셋 `KGBoneSkeletonData`(ScriptableObject)와 하위 에셋(묶은 텍스처, 블렌드 모드별 머티리얼)을 만든다.
- `KGBoneSkeletonData`에 담는 것: `editor` 블록을 뺀 skeleton JSON 텍스트, 이미지 경로 → 묶은 텍스처 안 사각형(UV) 표, 픽셀당 유닛(pixelsPerUnit), 가져오기 경고 목록. 런타임은 처음 쓸 때 JSON을 한 번 읽어 모델을 만들고, 같은 에셋을 쓰는 캐릭터끼리 공유한다.
  - 이유: 모델에는 이름 → 첨부물 같은 중첩 사전이 많아 유니티 직렬화로 옮기기 번거롭고, 데이터는 수 KB라 시작할 때 읽어도 부담이 없다.
- 이미지: `.kgbone` 기준 `imagesPath` 폴더(프로젝트 안이어야 함)에서 첨부물·스킨이 참조하는 PNG를 찾아 **한 장의 텍스처로 묶는다**(여백 2px). PNG는 파일에서 직접 읽으므로 원본 텍스처의 Read/Write 설정이 필요 없다. 각 PNG에 의존성을 걸어(`DependsOnSourceAsset`) 그림을 바꾸면 다시 가져온다.
- 가져오기 경고(`LogImportWarning`, 에셋에도 저장해 인스펙터에 표시): 제약이 있음, 클리핑 있음, 시퀀스 있음, 이미지 없음, imagesPath가 프로젝트 밖, 묶은 텍스처가 최대 크기를 넘음.
- 가져오기 설정(인스펙터): pixelsPerUnit(기본 100), 텍스처 최대 크기(기본 4096), 필터 모드.

### 5.4 재생 컴포넌트 (`Runtime/`)

`KGBoneAnimation` (MonoBehaviour, `MeshFilter` + `MeshRenderer` 필요)

| 인스펙터 | 기본값 |
|---|---|
| Skeleton Data | (필수) |
| Skin | 없음 = 기본 스킨 |
| Animation | 없음 = 셋업 포즈 |
| Loop | 켬 |
| Time Scale | 1 |
| Play On Awake | 켬 |
| Flip X / Flip Y | 끔 |
| Sorting Layer / Order in Layer | 렌더러 설정을 그대로 노출 |

코드 API (이름은 만드는 사람이 정해도 되지만 기능은 아래와 같다)
- `Play(string animation, bool loop)`, `Stop()`(셋업 포즈로), `Time`(읽기·쓰기), `TimeScale`, `IsPlaying`, `CurrentAnimation`
- `event Action<KGBoneEvent> EventFired` (이름, int, float, string, volume, balance, 시간), `event Action Completed` (반복이 아닐 때 끝에 도달)
- 본 위치 읽기: `TryGetBone(string name, out Vector3 position, out float rotation, out Vector2 scale)` — 유니티 월드 좌표(이 GameObject의 Transform, 뒤집기, pixelsPerUnit 반영)
- `KGBoneBoneFollower` 컴포넌트: 지정한 본을 매 프레임 따라가는 GameObject용(이펙트·무기 붙이기). 위치만 / 위치+회전 선택.
- 에디터에서 재생하지 않아도 셋업 포즈(또는 고른 애니메이션의 0초)가 씬 뷰에 보인다.

매 프레임: 시간 진행 → 계산부 실행 → 그리기 목록을 Mesh에 채움. Mesh·배열은 다시 쓰고 매 프레임 새로 할당하지 않는다.

### 5.5 그리기 (`Runtime/` + `Shaders/`)

- 그리기 목록을 순서대로 하나의 Mesh에 넣는다. **연속된 같은 블렌드 모드끼리 서브메시 하나**로 묶고, 서브메시 순서 = 그리기 순서. 블렌드 모드마다 머티리얼(가져오기 때 만든 4개 중 필요한 것)을 서브메시에 건다.
- 좌표: KGBone 단위(px, y 위) ÷ pixelsPerUnit → 유니티 로컬. 뒤집기는 정점 좌표 부호를 바꿔서 한다(셰이더는 양면 그리기).
- UV: 묶은 텍스처 안 사각형으로 옮기고, KGBone의 v(아래로 증가)를 유니티의 v(위로 증가)로 뒤집는다.
- 색: 정점 색 = 그리기 항목 색. 어두운 색은 두 번째 정점 데이터(예: UV 채널)로 넘긴다.
- 셰이더 `KGBone/Unlit` (URP, 투명 큐, 깊이 쓰기 끔, 양면): 텍스처 × 정점 색, 어두운 색이 있으면 tint black 공식. 블렌드 함수는 머티리얼 속성으로 4종(normal, additive, multiply, screen).
- 1단계에서 정확히 어떤 tint black·블렌드 공식을 쓸지는 KGBone 화면 렌더러(`app/src/render/blend.ts`, 셰이더)와 같게 맞춘다.

### 5.6 정답 비교 테스트

1. **정답 생성기** `tools/unity-golden.mjs` (`npm run unity:golden`): `kgbone.wasm`을 Node로 불러(MCP 서버와 같은 방식) 시험 파일마다 정해진 시점들의 `PoseOutput`(bones, active, draw, events)을 계산해 `unity/com.kgbone.runtime/Tests/Golden/*.json`에 쓴다. 시점은 애니메이션마다 0, 길이의 1/3, 1/2, 2/3, 길이, 반복 재생으로 길이를 넘긴 시점, 이벤트 구간용 (last, time) 쌍을 포함한다. 스킨이 여러 개면 스킨별로.
2. **대상 파일**: 제약·클리핑·시퀀스가 없는 시험 파일만.
   - 지금 있는 것: `samples/kgbone/bones-only.kgbone`, `samples/kgbone/notation.kgbone`, `samples/spine43/`의 `bones-only`, `mesh-weights`, `skins`, `events`. Spine JSON 호환 파일은 생성기가 핵심으로 가져와 `.kgbone`으로 바꿔 Golden 폴더에 함께 둔다.
   - 제외: `full-4.3.kgbone`(제약 4개), `character-4.2`(제약 2개).
   - **새 시험 파일** `samples/kgbone/unity-stage1.kgbone`을 `tools/gen-samples.mjs`에 추가한다. 지금 파일들이 덮지 못하는 것(상속 `noScaleOrReflection`, 링크드 메시, 블렌드 multiply·screen, `rgba2`·`rgb2` 키)을 포함해 1단계 지원 기능을 하나의 파일에서 모두 쓴다: 상속 5종 + inherit 키, 회전·크기가 있는 region, 비가중치·가중치 메시 + deform 키, 링크드 메시, 블렌드 4종, 어두운 색 + rgba2·rgb2 키, 첨부물 키, 그리기 순서 키, 스킨 2개 + 스킨 전용 본, 이벤트, 계단·베지어 곡선. 이미지도 생성기가 그린다.
3. **비교**: `unity/tests/KGBone.Core.Tests`(`dotnet test`)가 같은 파일을 C# 계산부로 계산해 항목별로 비교한다. 오차 1e-3, 정수·문자열(슬롯, 첨부물, 이미지, 블렌드, 인덱스, 이벤트)은 완전히 같아야 한다. 같은 비교를 유니티 EditMode 테스트로도 돌릴 수 있게 한다.
4. **CI**: GitHub Actions에 작업을 추가해 wasm 빌드 → `npm run unity:golden` → 정답 파일이 저장소 것과 같은지 확인(다르면 실패: 핵심이 바뀌었는데 정답을 갱신하지 않은 것) → `dotnet test`.
5. **유니티 안 확인**(이 PC에서만): 가져오기가 경고·하위 에셋을 제대로 만드는지, 컴포넌트가 재생·이벤트·본 위치 읽기를 하는지 EditMode 테스트. 실제 화면은 `KGBoneSandbox`에서 눈으로 확인한다.
   - `KGBoneSandbox/Assets/Samples/`는 커밋하지 않고(`.gitignore`), 도우미 명령(`npm run unity:sandbox`)이 `samples/`의 시험 파일과 이미지를 복사해 채운다. 저장소에 같은 파일이 두 벌 생기지 않게 하려는 것이다. 골렘처럼 저장소에 넣을 수 없는 파일은 사용자가 직접 이 폴더에 넣어 확인한다.

### 5.7 문서와 규칙

- `docs/SPEC.md`에 "M. 유니티 런타임" 절을 추가하고 번호(M01…)마다 완료 기준을 적는다. 테스트 이름에 번호를 넣는다(예: `m03_region_corners_match_core`).
- `CLAUDE.md`: 폴더 구조에 `unity/`, 3장에 규칙 예외(5장 명령 `npm run unity:golden`, `dotnet test unity/tests/KGBone.Core.Tests`), 6장 '완료'에 유니티 테스트 포함.
- `docs/GAPS.md`: 1단계 비목표를 적는다.
- `README.md`와 패키지 `README.md`: 설치 주소, 가져오기, 컴포넌트 사용법, 지원·미지원 기능.
- 패키지 버전 태그는 `unity-v0.1.0`처럼 `unity-` 접두사를 붙여 에디터와 따로 매긴다.

## 6. 설계자가 정한 것 (사용자 확인 필요)

사용자가 직접 정하지 않았고 설계 중에 제가 정한 것들이다. 다르게 원하면 고친다.

| 항목 | 정한 값 | 이유 |
|---|---|---|
| 데이터 저장 방식 | 편집 상태를 뺀 JSON 텍스트를 에셋에 담고 처음 쓸 때 읽는다 | 중첩 사전이 많아 유니티 직렬화가 번거롭고, 데이터가 수 KB라 부담이 없다 |
| pixelsPerUnit 기본값 | 100 | 유니티 스프라이트 기본값과 같아 다른 스프라이트와 크기가 맞는다 |
| 텍스처 묶기 | 가져올 때 한 장으로(여백 2px, 최대 4096) | 그리기 호출을 줄인다 |
| 본 따라가기 컴포넌트 | 1단계에 포함 | "본 위치 읽기"의 가장 흔한 용도(이펙트·무기 붙이기)가 코드 없이 된다 |
| 지원 안 하는 기능 | 제약은 무시, 클리핑은 자르지 않고, 시퀀스는 셋업 프레임으로 그리며 모두 경고 | 아예 못 여는 것보다 낫고, 경고로 차이를 알린다 |
| 패키지 태그 | `unity-v0.1.0` | 에디터와 버전을 따로 매긴다 |
| 확인용 유니티 프로젝트 | `unity/KGBoneSandbox/` 커밋, 시험 파일은 복사로 채움 | 손으로 확인하는 장소를 저장소에 남기되 파일 중복은 피한다 |

## 7. 만드는 사람에게 맡기는 것

- 클래스·파일 이름, 파일 나누기(CLAUDE.md 3장 원칙대로), 계산부 내부 자료 구조
- JSON 파서 구현 방식
- 텍스처 묶는 방법(유니티 `Texture2D.PackTextures` 등)
- 테스트 프레임워크(NUnit 권장: 유니티 테스트와 같음)와 .NET 대상 버전
- 서브메시·머티리얼 캐시, 메모리 재사용 방식
- 정답 생성 시점의 정확한 목록(위 범위를 덮으면 됨)
