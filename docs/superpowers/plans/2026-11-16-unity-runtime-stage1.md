# KGBone 유니티 런타임 1단계 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `.kgbone` 파일을 유니티(Unity 6 + URP)에서 KGBone 에디터와 같은 결과로 재생하는 UPM 패키지 `com.kgbone.runtime`을 만든다(1단계: 재생 + 본 위치 읽기).

**Architecture:** 순수 C# 계산부(`Runtime/Core`, UnityEngine 없음)가 KGBone 핵심(Rust)의 애니메이션 적용·본 월드 계산·그리기 목록을 같은 규칙으로 옮긴다. 정답은 `kgbone.wasm`으로 만든 정답 파일이고, `dotnet test`가 계산부 결과를 비교한다. 유니티부는 `.kgbone` 가져오기 도구(ScriptedImporter, 텍스처 묶기)와 재생 컴포넌트(매 프레임 Mesh 하나를 다시 채움), URP용 셰이더다.

**Tech Stack:** C# 9(유니티 호환), .NET SDK 10 + NUnit(계산부 테스트), Unity 6000.0+ / URP, Node 24(정답 생성기, `tools/mcp/core.mjs` 재사용)

**Spec:** `docs/superpowers/specs/2026-11-16-unity-runtime-stage1-design.md`

## Global Constraints

- 위치: 저장소 `unity/com.kgbone.runtime/` (package.json `"name": "com.kgbone.runtime"`, `"version": "0.1.0"`, `"unity": "6000.0"`).
- 클린룸: 근거는 `docs/FORMAT.md`와 `crates/kgbone-core` 코드뿐. `EsotericSoftware/spine-runtimes`의 어떤 파일도 열지 않는다. 패키지·네임스페이스·클래스·메뉴 이름에 "Spine"을 쓰지 않는다.
- 계산부 `Runtime/Core`: asmdef `KGBone.Runtime.Core`, `"noEngineReferences": true`, 외부 패키지 의존성 0개. **C# 9 문법만**(파일 범위 네임스페이스, global using, `required`, 컬렉션 식, record struct 금지). 테스트 프로젝트에 `<LangVersion>9.0</LangVersion>`을 걸어 같은 제약으로 컴파일한다.
- 숫자: 핵심과 같은 32비트 `float`, 삼각함수는 `MathF`. 베지어 풀이만 핵심처럼 `double`(60회 이분법).
- 정답 비교 오차: 실수 1e-3(절대값), 문자열·정수·참거짓·인덱스는 완전히 같음.
- 1단계 미지원(제약, 클리핑, 시퀀스)은 계산하지 않고 가져올 때 경고한다. 제약은 무시, 클리핑은 자르지 않고 그림, 시퀀스는 셋업 프레임(`setup`) 이미지로 그린다.
- 렌더: URP, 투명 큐, 깊이 쓰기 끔, 양면. 텍스처는 미리 곱한 알파. 색 공식은 화면과 같게 `rgb = (t.rgb × light.rgb + (t.a − t.rgb) × dark) × light.a`, `a = t.a × light.a`. 블렌드: normal `One, OneMinusSrcAlpha` / additive `One, One` / multiply `DstColor, OneMinusSrcAlpha` / screen `One, OneMinusSrcColor` (`app/src/render/blend.ts`).
- 가져오기 기본값: pixelsPerUnit 100, 텍스처 최대 4096, 여백 2px.
- 커밋: `[ADD]/[CHG]/[DEL]/[FIX]` + 한국어. 공동 작업자 줄 금지. 작업마다 `git push origin HEAD:main`(강제 push 금지). `.gitignore`의 `.claude/` 항목 유지.
- 테스트 이름에 SPEC 번호(`m01`…`m12`, Task 13에서 SPEC.md에 적는 번호)를 넣는다.
- 저장소 라이선스는 아직 없다. 패키지 `LICENSE.md`와 버전 태그는 만들지 않는다(배포 전 사용자 결정).

## Review Focus

1. **이미지가 없거나 하위 폴더·한글 이름인 경로**(`팔/왼쪽`): 가져오기는 경고만 하고 나머지 파츠는 그려야 한다. → Task 10에 테스트.
2. **없는 애니메이션 이름으로 `Play`**: 예외 없이 경고 로그, 지금 재생 상태 유지. `Play(null)`은 셋업 포즈. → Task 11에 테스트.
3. **반복 끔 + 끝을 지남**: 마지막 프레임에 멈추고 `Completed`는 한 번만. → Task 11에 테스트.
4. **한 프레임에 반복을 여러 바퀴 지나는 큰 dt(멈칫)**: 바퀴마다 이벤트가 빠짐없이 순서대로 난다. → Task 7에 테스트.
5. **길이 0인 애니메이션을 반복 재생**: 시점 0, 0으로 나누기 없음, 이벤트는 0초 키만. → Task 7에 테스트.
6. 데이터가 비어 있는 컴포넌트(에디터에서 막 붙인 직후)는 예외 없이 아무것도 그리지 않는다. → Task 11에 테스트.

---

## 파일 구조

```
unity/com.kgbone.runtime/
  package.json
  README.md, CHANGELOG.md
  Runtime/Core/KGBone.Runtime.Core.asmdef
    Json/JsonValue.cs, Json/JsonParser.cs                 JSON 읽기
    Model/SkeletonModel.cs, BoneData.cs, SlotData.cs, SkinData.cs, AttachmentData.cs,
          AnimationData.cs, Key.cs, ColorF.cs, Enums.cs    모델(FORMAT.md 3장)
    Model/KGBoneLoader.cs                                  .kgbone 텍스트 → 모델, 경고 수집
    Anim/Curve.cs                                          value_at, ratio_at, bezier
    Anim/AnimationApplier.cs                               본·슬롯·그리기 순서·deform 키
    Pose/WorldBone.cs, Pose/BoneTransform.cs, Pose/ActiveBones.cs
    Pose/PoseState.cs                                      셋업 상태(SkeletonPose 대응)
    Pose/MeshVertices.cs                                   region 모서리, 메시 정점, resolve_mesh
    Pose/DrawListBuilder.cs, Pose/EventFinder.cs
    Pose/PoseRequest.cs, Pose/PoseResult.cs, Pose/SkeletonPoser.cs   바깥 입구
  Runtime/KGBone.Runtime.asmdef
    KGBoneSkeletonData.cs, KGBoneAnimation.cs, KGBoneEvent.cs, KGBoneBoneFollower.cs, MeshBuilder.cs
  Shaders/KGBoneUnlit.shader
  Editor/KGBone.Editor.asmdef
    KGBoneImporter.cs, TexturePacker.cs, KGBoneSkeletonDataEditor.cs
  Tests/Golden/*.json, *.kgbone                            정답 파일(생성기 출력)
  Tests/Editor/KGBone.Tests.asmdef
    GoldenTests.cs, GoldenFile.cs                          dotnet·유니티 공용(순수 C#)
    CoreUnitTests.cs                                       곡선·상속·그리기 순서 등 손 계산
    ImporterTests.cs, AnimationComponentTests.cs           유니티 전용
unity/tests/KGBone.Core.Tests/KGBone.Core.Tests.csproj     Core + Tests/Editor의 공용 파일을 링크
unity/KGBoneSandbox/                                       확인용 유니티 프로젝트
tools/unity-golden.mjs, tools/unity-sandbox.mjs
.github/workflows/unity-core.yml
```

---

### Task 1: 패키지 뼈대와 .NET 테스트 실행 확인

**Files:**
- Create: `unity/com.kgbone.runtime/package.json`, `Runtime/Core/KGBone.Runtime.Core.asmdef`, `Runtime/KGBone.Runtime.asmdef`, `Editor/KGBone.Editor.asmdef`, `Tests/Editor/KGBone.Tests.asmdef`, `Runtime/Core/Pose/WorldBone.cs`
- Create: `unity/tests/KGBone.Core.Tests/KGBone.Core.Tests.csproj`, `unity/tests/KGBone.Core.Tests/SmokeTests.cs`
- Modify: `.gitignore`, `package.json`(루트, 스크립트)

**Interfaces:**
- Produces: `namespace KGBone.Core { public struct WorldBone { public float A, B, C, D, X, Y; public static readonly WorldBone Identity; public (float x, float y) LocalToWorld(float lx, float ly); } }`
- Produces: 루트 스크립트 `"unity:test": "dotnet test unity/tests/KGBone.Core.Tests"`

- [ ] **Step 1:** package.json 작성. `dependencies`에 `"com.unity.render-pipelines.universal"`을 넣되 버전은 Task 12에서 Sandbox가 실제로 받는 Unity 6000.0 대응 버전으로 확정한다(지금은 `"17.0.3"`으로 두고 Task 12 Step 1에서 검증).
- [ ] **Step 2:** asmdef 4개. Core: `noEngineReferences: true`. Runtime: Core 참조. Editor: `includePlatforms: ["Editor"]`, Core·Runtime 참조. Tests: `includePlatforms: ["Editor"]`, `defineConstraints: ["UNITY_INCLUDE_TESTS"]`, `overrideReferences: true`, `precompiledReferences: ["nunit.framework.dll"]`, `optionalUnityReferences: ["TestAssemblies"]`, Core·Runtime·Editor 참조.
- [ ] **Step 3:** csproj: `net10.0`, `<LangVersion>9.0</LangVersion>`, `<Nullable>disable</Nullable>`, NUnit·NUnit3TestAdapter·Microsoft.NET.Test.Sdk, `<Compile Include="../../com.kgbone.runtime/Runtime/Core/**/*.cs" />`, `<Compile Include="../../com.kgbone.runtime/Tests/Editor/Golden*.cs;../../com.kgbone.runtime/Tests/Editor/CoreUnitTests.cs" />`(아직 없는 파일은 와일드카드라 괜찮다).
- [ ] **Step 4: 실패하는 테스트** `SmokeTests.m01_world_bone_identity_maps_point`: `WorldBone.Identity.LocalToWorld(3, 4)` == `(3, 4)`.
- [ ] **Step 5:** `npm run unity:test` 실행 → 컴파일 실패 확인 → `WorldBone` 작성 → 다시 실행해 통과.
  - **이 PC에서 스마트 앱 컨트롤이 막으면**(os error 4551 또는 "애플리케이션 제어 정책" 메시지): 그 출력을 `docs/GAPS.md`에 적고, 이후 계산부 테스트는 CI(Task 9)와 유니티 EditMode(Task 12)로 확인한다. 계획은 그대로 진행한다.
- [ ] **Step 6:** `.gitignore`에 `unity/tests/**/bin/`, `unity/tests/**/obj/`, `unity/KGBoneSandbox/Library/`, `Temp/`, `Logs/`, `UserSettings/`, `obj/`, `unity/KGBoneSandbox/Assets/Samples/`, `unity/KGBoneSandbox/Assets/Samples.meta` 추가.
- [ ] **Step 7:** 커밋 `[ADD] 유니티 런타임 패키지 뼈대와 계산부 테스트 프로젝트 추가` 후 push.

### Task 2: JSON 읽기

**Files:** Create `Runtime/Core/Json/JsonValue.cs`, `JsonParser.cs`; Test `Tests/Editor/CoreUnitTests.cs`

**Interfaces:**
- Produces: `public sealed class JsonValue { public JsonKind Kind; public static JsonValue Parse(string text); public JsonValue Get(string key); /* 없으면 null */ public IReadOnlyList<JsonValue> Items; public IReadOnlyList<KeyValuePair<string, JsonValue>> Members; /* 파일 순서 */ public float AsFloat(float fallback = 0); public int AsInt(int fallback = 0); public bool AsBool(bool fallback = false); public string AsString(string fallback = null); public float[] AsFloatArray(); }`, `public enum JsonKind { Null, Bool, Number, String, Array, Object }`
- Produces: `public sealed class KGBoneFormatException : System.Exception` (한국어 메시지, 줄·열 포함)

- [ ] **Step 1: 실패하는 테스트** `m02_json_parses_nested_ordered_members`: `{"b":1,"a":[1.5,-2e1,"한글\u0041"],"c":{"t":true,"n":null}}` → Members 키 순서 `b,a,c`, `Get("a").Items[1].AsFloat() == -20`, `Items[2].AsString() == "한글A"`, `Get("c").Get("n").Kind == Null`. `m02_json_error_reports_position`: `{"a":}` → `KGBoneFormatException`, 메시지에 `1:6` 포함.
- [ ] **Step 2:** 실행해 실패 확인 → 재귀 하강 파서 작성(숫자는 `double.Parse(…, CultureInfo.InvariantCulture)` 후 보관, BOM 무시) → 통과.
- [ ] **Step 3:** 커밋 `[ADD] 유니티 런타임 JSON 읽기 추가` 후 push.

### Task 3: 모델과 `.kgbone` 읽기

**Files:** Create `Runtime/Core/Model/*.cs`, `Model/KGBoneLoader.cs`, `Tests/Editor/GoldenFile.cs`(이 작업에서는 `RepoRoot()`만); Test `CoreUnitTests.cs`

**Interfaces:**
- Produces (모두 `KGBone.Core`):
  - `enum Inherit { Normal, OnlyTranslation, NoRotationOrReflection, NoScale, NoScaleOrReflection }`, `enum BlendMode { Normal, Additive, Multiply, Screen }`, `enum CurveType { Linear, Stepped, Bezier }`
  - `struct ColorF { public float R, G, B, A; public static ColorF Parse(string hex); /* "RRGGBBAA" 또는 "RRGGBB"(A=1), 각 채널 /255 */ }`
  - `class BoneData { Name; int Parent /* -1 */; Length, X, Y, Rotation, ScaleX, ScaleY, ShearX, ShearY; Inherit Inherit; bool SkinRequired; }`
  - `class SlotData { Name; int Bone; ColorF Color; ColorF? Dark; string Attachment; BlendMode Blend; }`
  - `abstract class AttachmentData { string Path; /* 없으면 null */ public string ImagePath(string key); /* Path ?? key, 그림이 없는 종류는 null */ }`, `RegionAttachment { X, Y, Rotation, ScaleX, ScaleY, Width, Height; ColorF Color; }`, `MeshAttachment { Width, Height; ColorF Color; float[] Uvs; int[] Triangles; float[] Vertices; bool Weighted; }`, `LinkedMeshAttachment { ColorF Color; string Parent; string Skin; bool Timelines; }`, `OtherAttachment { string Type; }`
  - `class SkinData { Name; List<string> Bones; public AttachmentData Get(string slot, string name); }`
  - `class Key { float Time; float[] V; string S; CurveType Curve; float[] Bezier; int Offset; }`
  - `class AnimationData { Name; List<BoneTimeline> Bones; List<SlotTimeline> Slots; List<DeformTimeline> Deform; List<DrawOrderKey> DrawOrder; List<EventKey> Events; float Duration; }` — `BoneTimeline { int Bone; string Name; Key[] Keys; }`, `SlotTimeline { int Slot; string Name; Key[] Keys; }`, `DeformTimeline { string Skin, Slot, Attachment; Key[] Keys; }`, `DrawOrderKey { float Time; (string Slot, int Offset)[] Offsets; }`, `EventKey { float Time; string Name; int? Int; float? Float; string String; float? Volume, Balance; }`
  - `class EventData { Name; int Int; float Float; string String; float Volume /*1*/; float Balance; }`
  - `class SkeletonModel { List<BoneData> Bones; List<SlotData> Slots; List<SkinData> Skins; List<EventData> Events; List<AnimationData> Animations; float Fps; int ConstraintCount; bool HasClipping; bool HasSequence; int BoneIndex(string); int SlotIndex(string); SkinData FindSkin(string); AnimationData FindAnimation(string); string ImagesPath; }`
  - `static class KGBoneLoader { public static SkeletonModel Load(string kgboneText); public static IEnumerable<string> ImagePaths(SkeletonModel m); /* 모든 스킨의 그림 첨부물 이미지 경로, 중복 없음 */ }`
- 기본값은 FORMAT.md 3장 표 그대로. `Duration`은 `model/animation.rs`의 `duration()`과 같게(본·슬롯·제약·deform·sequence의 마지막 키, 그리기 순서·이벤트 키 중 최대). 제약 타임라인과 sequence 키는 길이 계산에만 쓰고 버린다.
- 이미지 경로는 시퀀스가 있으면 핵심처럼 `path + (start + setup)`을 `digits`자리로 붙인 이름으로 바꿔 둔다(`pose/draw.rs`의 `sequence_path`, 프레임 = setup).

- [ ] **Step 1: 실패하는 테스트** `m02_loader_reads_samples`: Task 4가 만들 Golden 폴더가 아직 없으므로 저장소의 `samples/kgbone/full-4.3.kgbone`을 경로로 읽어 `Bones[0].Name == "root"`, `ConstraintCount == 4`, 본 `몸통`의 `Rotation == 90`, `Animations` 이름에 `꼬리흔들기` 포함, 슬롯 `몸`의 셋업 첨부물 `body`가 `RegionAttachment`이고 `Width == 48`. `m02_loader_defaults`: `{"formatVersion":1,"skeleton":{"bones":[{"name":"root"}]}}` → ScaleX 1, Inherit Normal, ImagesPath `"./images"`, Fps 30.
- [ ] **Step 2:** 실행해 실패 확인 → 작성 → 통과. (테스트는 `GoldenFile.RepoRoot()`로 저장소 루트를 찾는다: `TestContext.CurrentContext.TestDirectory`에서 위로 올라가며 `.git`이 있는 폴더. 워크트리에서는 `.git`이 파일이므로 파일·폴더 둘 다 인정한다.)
- [ ] **Step 3:** 커밋 `[ADD] 유니티 런타임 모델과 .kgbone 읽기 추가` 후 push.

### Task 4: 1단계 시험 파일과 정답 생성기

**Files:** Modify `tools/gen-samples.mjs`, `Tests/Editor/GoldenFile.cs`(GoldenDir, All 추가); Create `tools/unity-golden.mjs`, `Tests/Golden/*`; Modify 루트 `package.json`

**Interfaces:**
- Produces: `samples/kgbone/unity-stage1.kgbone` (+ 이미지는 `samples/images/`에 생성기가 그림)
- Produces: 정답 파일 `Tests/Golden/<이름>.json`:
  ```json
  { "kgbone": "<이름>.kgbone", "source": "samples/…", "cases": [
    { "request": { "animation": "걷기"|null, "time": 0.5, "lastTime": 0.25|null, "skin": "빨강"|null, "looping": true },
      "output": { "bones": [{"a":…,"b":…,"c":…,"d":…,"x":…,"y":…}], "active": [true],
                  "draw": [{"slot":…,"attachment":…,"image":…,"blend":"normal","color":[r,g,b,a],"dark":[r,g,b]|null,"positions":[…],"uvs":[…],"indices":[…]}],
                  "events": [{"name":…,"time":…,"int":…,"float":…,"string":…,"volume":…,"balance":…}] } } ] }
  ```
  `output`은 핵심 `pose` 응답(`interface/core-api.md`)에서 `locals`·`overlays`를 뺀 것. 요청의 `physics`는 항상 `"none"`.
- Produces: `public static class GoldenFile { public static string RepoRoot(); public static string GoldenDir(); /* 패키지 Tests/Golden 절대 경로: dotnet은 RepoRoot 기준, 유니티는 Path.GetFullPath("Packages/com.kgbone.runtime/Tests/Golden") */ public static IEnumerable<string> All(); }`
- Produces: 루트 스크립트 `"unity:golden": "node tools/unity-golden.mjs"`

- [ ] **Step 1:** `gen-samples.mjs`에 `unity-stage1.kgbone` 추가. 반드시 담을 것: 상속 5종 본 각 1개 이상 + `inherit` 키, 회전 30·scaleX 2인 region, 비가중치 메시 + deform 키(offset 2, 베지어 곡선), 가중치 메시(본 2개), 링크드 메시(`timelines: true`), 슬롯 블렌드 4종, 어두운 색 슬롯 + `rgba2`·`rgb2` 키, 슬롯 `attachment` 키(비움 포함), `drawOrder` 키 2개, 스킨 `default` + `빨강`(`skinRequired` 본 1개, 그 본에 붙은 슬롯 1개), 이벤트 정의 + 키 3개(0초 포함), 계단·베지어 곡선 키. 애니메이션 2개(`걷기` 길이 1, `정지` 키 한 개로 길이 0).
- [ ] **Step 2:** `npm run samples` → 새 파일 생성 확인 → 기존 Rust 테스트가 시험 파일 목록을 세는 곳이 있으면 함께 고친다(`npm run test:rust`는 CI에서 확인).
- [ ] **Step 3:** `unity-golden.mjs`: `tools/mcp/core.mjs`의 `loadCore`로 wasm을 읽고, 대상 목록(설계 문서 5.6: `samples/kgbone/bones-only`, `notation`, `unity-stage1`, `samples/spine43/bones-only`, `mesh-weights`, `skins`, `events`)마다 `.kgbone`은 `openKgbone`, `.json`은 `importSpineJson` 후 `saveKgbone` 텍스트를 `Tests/Golden/<이름>.kgbone`으로 저장. 케이스: 스킨(없음 + 정의된 스킨 각각) × (셋업 포즈 + 애니메이션마다 시점 `0, d/3, d/2, 2d/3, d, 1.5d(looping)`, 이벤트 쌍 `(lastTime 0→d/2)`, `(0.9d→1.2d looping)`, `(0.9d→0.1d looping)`). 숫자는 `JSON.stringify` 그대로, 파일은 키 순서 고정·탭 들여쓰기·끝 줄바꿈으로 결정적으로 쓴다.
- [ ] **Step 4:** `npm run wasm`(Rust가 없으면 `npm run wasm:fetch`) → `npm run unity:golden` → 같은 명령을 한 번 더 돌려 `git diff --stat` 변화 없음(결정적) 확인.
- [ ] **Step 5:** `GoldenFile.cs`에 `GoldenDir()`, `All()` 추가(순수 C#, `System.IO`만).
- [ ] **Step 6:** 커밋 `[ADD] 유니티 런타임 1단계 시험 파일과 정답 생성기 추가` 후 push.

### Task 5: 곡선

**Files:** Create `Runtime/Core/Anim/Curve.cs`; Test `CoreUnitTests.cs`

**Interfaces:**
- Produces: `static class Curve { public static float ValueAt(Key[] keys, int channel, float time); public static bool RatioAt(Key[] keys, float time, out int index, out float ratio); public static int StepIndex(Key[] keys, float time); /* 없으면 -1 */ public static float Bezier(float t0, float v0, float t1, float v1, float cx1, float cy1, float cx2, float cy2, float time); }` — `anim/curve.rs`와 같은 규칙(첫 키 앞·마지막 뒤는 끝 값, 이분 탐색, 채널 곡선이 없으면 선형, 베지어 제어점 시간을 [t0, t1]로 자르고 double 60회 이분법).

- [ ] **Step 1: 실패하는 테스트** `m04_curve_linear_stepped_bezier`: 키 `(0,[0]) linear (1,[10])`에서 0.25 → 2.5, `-1` → 0, `2` → 10; 계단이면 0.99 → 0; 베지어 `[0.3,0,0.7,20]`(키 0→1, 값 0→20)에서 0.5 → 10(대칭) ±1e-3, 채널 1 곡선이 없으면 선형. `m04_ratio_uses_first_channel`: 베지어 `[0.25,0,0.75,1]` 0.5 → 0.5.
- [ ] **Step 2:** 실패 확인 → 작성 → 통과 → 커밋 `[ADD] 유니티 런타임 곡선 계산 추가` 후 push.

### Task 6: 본 월드 계산과 활성 판정

**Files:** Create `Pose/BoneTransform.cs`, `Pose/ActiveBones.cs`, `Pose/PoseState.cs`; Test `CoreUnitTests.cs`

**Interfaces:**
- Produces: `struct LocalTransform { X, Y, Rotation, ScaleX, ScaleY, ShearX, ShearY }`
- Produces: `static class BoneTransform { public static WorldBone WorldOf(in LocalTransform l, Inherit inherit, WorldBone? parent); public static void ComputeAll(SkeletonModel m, LocalTransform[] locals, Inherit[] inherits, WorldBone[] outWorld); }` — `pose/bone_transform.rs`의 `local_matrix`, `inherited_matrix`(5종, θ = atan2(c, a), 반사 = det < 0), `world_of`와 같게.
- Produces: `static class ActiveBones { public static void Compute(SkeletonModel m, string skin, bool[] outActive); }` — `pose/active.rs`의 `active_bones`.
- Produces: `sealed class PoseState { LocalTransform[] Locals; Inherit[] Inherits; WorldBone[] World; bool[] Active; ColorF[] SlotColors; ColorF?[] SlotDarks; string[] SlotAttachments; int[] DrawOrder; Dictionary<(string slot, string att), float[]> Deform; public PoseState(SkeletonModel m); public void ResetToSetup(SkeletonModel m); }` (배열은 한 번 만들고 다시 쓴다)

- [ ] **Step 1: 실패하는 테스트** `m03_inherit_modes_hand_computed`: 부모 (회전 90, scaleX 2, 위치 (10,20)), 자식 로컬 (5,0, 회전 0). normal → 월드 위치 (10, 30), a≈0, c≈2; onlyTranslation → 위치 (10,30), 행렬 단위; noScale → c≈1(크기 빠짐); noRotationOrReflection → a≈2, c≈0; 부모 scaleX −1일 때 noScale과 noScaleOrReflection의 d 부호가 다름. `m08_skin_required_bone_inactive_without_skin`: skinRequired 본과 그 자식이 스킨 없음이면 false, 그 본을 가진 스킨이면 true.
- [ ] **Step 2:** 실패 확인 → 작성 → 통과 → 커밋 `[ADD] 유니티 런타임 본 월드 계산과 활성 판정 추가` 후 push.

### Task 7: 애니메이션 적용과 이벤트

**Files:** Create `Anim/AnimationApplier.cs`, `Pose/EventFinder.cs`; Test `CoreUnitTests.cs`

**Interfaces:**
- Produces: `static class AnimationApplier { public static void Apply(SkeletonModel m, AnimationData a, float time, string skin, PoseState pose); }` — 섞기 비율 1 고정. 본 키(`anim/bone_keys.rs`: rotate·translate·shear 더하기, scale 곱하기, 축별 이름, inherit 계단), 슬롯 키(`anim/slot_keys.rs`: rgba/rgb/alpha/rgba2/rgb2는 0~1로 자름, 어두운 색이 없던 슬롯은 셋업 어두운 색 (0,0,0)에서 시작, attachment 계단·`S` 없으면 비움), 그리기 순서(`anim/slot_keys.rs`의 `draw_order_of`와 `last_key_at` — `commands/draw_order.rs`의 decode가 아니다), deform(`anim/attachment_keys.rs`: 기본 스킨과 고른 스킨 것만, `expand` + `RatioAt`).
- Produces: `static class EventFinder { public static void Fired(SkeletonModel m, AnimationData a, float last, float time, bool looping, List<FiredEvent> output); }` — `pose/events.rs`의 `hits`와 `fired`(키에 없는 값은 이벤트 정의 기본값).
- Produces: `static float AnimationTime(float duration, float time, bool looping)` — 반복이면 `time` mod `duration`(음수도 양수 나머지), 길이 0이면 0.

- [ ] **Step 1: 실패하는 테스트** `m05_bone_keys_relative_to_setup`(셋업 회전 30 + 키 10 → 40, 셋업 scaleX 2 × 키 1.5 → 3), `m06_draw_order_key_moves_slot`(슬롯 a,b,c, 키 `{a:+2}` → 순서 [b,c,a]), `m06_attachment_key_without_s_clears`, `m09_events_half_open_and_looping`(events.rs 테스트와 같은 값: [0,0.5) → [0]; 반복 [0.9,1.2) → 2개; 0.9→0.1 반복 → [1.0, 0.0]), **Review Focus 4** `m09_large_dt_crossing_two_loops_fires_each_lap`(길이 1, 키 0·0.5, [0.25, 2.75) 반복 → 0.5, 1.0, 1.5, 2.0, 2.5 순서로 5개), **Review Focus 5** `m09_zero_duration_loop_is_time_zero`(`AnimationTime(0, 3.7, true) == 0`, 0초 키만 난다).
- [ ] **Step 2:** 실패 확인 → 작성 → 통과 → 커밋 `[ADD] 유니티 런타임 애니메이션 적용과 이벤트 추가` 후 push.

### Task 8: 그리기 목록과 바깥 입구, 정답 비교

**Files:** Create `Pose/MeshVertices.cs`, `Pose/DrawListBuilder.cs`, `Pose/PoseRequest.cs`, `Pose/PoseResult.cs`, `Pose/SkeletonPoser.cs`, `Tests/Editor/GoldenTests.cs`

**Interfaces:**
- Produces: `class PoseRequest { string Animation; float Time; float? LastTime; string Skin; bool Looping; }`
- Produces: `sealed class DrawItem { string Slot, Attachment, Image; BlendMode Blend; ColorF Color; ColorF? Dark; List<float> Positions; List<float> Uvs; List<int> Indices; }`, `struct FiredEvent { Name; Time; Int; Float; String; Volume; Balance; }`
- Produces: `sealed class PoseResult { WorldBone[] Bones; bool[] Active; List<DrawItem> Draw; List<FiredEvent> Events; }`
- Produces: `sealed class SkeletonPoser { public SkeletonPoser(SkeletonModel m); public SkeletonModel Model; public PoseResult Compute(PoseRequest r); /* 같은 PoseResult 인스턴스를 다시 채워 돌려준다 */ public int BoneIndex(string name); }` — 순서는 `pose/mod.rs`의 `compute_pose`에서 제약을 뺀 것: 셋업 → 애니메이션(`AnimationTime`) → 활성 → 월드 → 그리기 목록 → 이벤트(LastTime이 있을 때).
- 그리기(`pose/draw.rs`): region 모서리 순서·uv·삼각형 고정값, 메시는 `vertex_world_positions`(가중치 항목마다 deform 2개), 링크드 메시는 `resolve_mesh`(부모를 `Skin ?? "default"` 스킨에서, `Timelines`면 부모 이름의 deform), 색 = 슬롯 색 × 첨부물 색, 비활성 본의 슬롯·찾지 못한 첨부물은 건너뜀, 첨부물 찾기는 고른 스킨 → 기본 스킨. 그림이 없는 종류(boundingbox, point, path, clipping)는 그리지 않는다.

- [ ] **Step 1: 실패하는 테스트** `GoldenTests.m07_all_golden_cases_match_core`: `GoldenFile.All()`의 모든 파일, 모든 케이스를 `[TestCaseSource]`로 펼쳐 이름을 `<파일>/<번호>/<애니메이션>@<시점>`로. 비교: 본 개수·각 성분 1e-3, active 같음, 그리기 항목 개수·slot·attachment·image·blend 문자열 같음, color·dark·positions·uvs 1e-3, indices 같음, 이벤트 개수·이름·int·string 같음, time·float·volume·balance 1e-3. 다르면 첫 차이의 경로(`draw[3].positions[5]`)와 두 값을 메시지에 넣는다. 추가로 `m07_region_corners_hand_computed`(`pose/draw.rs`의 `f_d01_region_corners_world`와 같은 값).
- [ ] **Step 2:** `npm run unity:test`(또는 막힌 PC면 CI) 실패 확인 → 작성 → 모든 정답 케이스 통과.
- [ ] **Step 3:** 커밋 `[ADD] 유니티 런타임 그리기 목록과 정답 비교 테스트 추가` 후 push.

### Task 9: CI

**Files:** Create `.github/workflows/unity-core.yml`

- [ ] **Step 1:** ubuntu-latest, main push·PR·workflow_dispatch. 단계: checkout → setup-node 24 → Rust(stable + `wasm32-unknown-unknown`) → `npm ci` → `npm run wasm` → `npm run unity:golden` → `git diff --exit-code unity/com.kgbone.runtime/Tests/Golden samples` (실패 메시지: "정답 파일이 핵심과 다릅니다. npm run samples && npm run unity:golden 후 커밋하세요") → setup-dotnet 10 → `npm run unity:test`.
- [ ] **Step 2:** push 후 Actions 결과 확인(성공해야 다음 작업). 커밋 `[ADD] 유니티 런타임 계산부 CI 추가`.

### Task 10: 데이터 에셋과 가져오기 도구

**Files:** Create `Runtime/KGBoneSkeletonData.cs`, `Editor/KGBoneImporter.cs`, `Editor/TexturePacker.cs`, `Editor/KGBoneSkeletonDataEditor.cs`, `Tests/Editor/ImporterTests.cs`

**Interfaces:**
- Produces: `public sealed class KGBoneSkeletonData : ScriptableObject { [SerializeField] string skeletonJson; [SerializeField] Texture2D texture; [SerializeField] Material[] materials; /* 길이 4, BlendMode 순서, 안 쓰는 모드는 null */ [SerializeField] ImageEntry[] images; [SerializeField] float pixelsPerUnit; [SerializeField] string[] warnings; public Texture2D Texture; public float PixelsPerUnit; public IReadOnlyList<string> Warnings; public SkeletonModel Model; /* 처음 읽을 때 KGBoneLoader.Load, 캐시 */ public Material MaterialFor(BlendMode m); public bool TryGetImage(string path, out Rect uv, out Vector2 sizePx); }`, `[Serializable] public struct ImageEntry { public string path; public Rect uv; public Vector2 size; }`
- Produces: `[ScriptedImporter(1, "kgbone")] public sealed class KGBoneImporter : ScriptedImporter { public float pixelsPerUnit = 100; public int maxTextureSize = 4096; public FilterMode filterMode = FilterMode.Bilinear; }`
- `skeletonJson`에는 최상위에서 `editor`를 뺀 JSON(`formatVersion`, `imagesPath`, `skeleton`)만 담는다.
- 텍스처: `KGBoneLoader.ImagePaths`의 각 경로를 `<.kgbone 폴더>/<imagesPath>/<경로>.png`로 찾아 `ImageConversion.LoadImage`로 읽고, **알파를 미리 곱한 뒤** 묶는다(여백 2px, 최대 크기 넘으면 경고 후 축소 허용). 각 PNG에 `ctx.DependsOnSourceAsset`. 하위 에셋: 텍스처 1장, 필요한 블렌드 모드 머티리얼(`KGBone/Unlit`, 블렌드 속성 설정).
- 경고 문구(그대로): `제약 {n}개는 1단계 런타임에서 계산하지 않습니다`, `클리핑은 1단계 런타임에서 자르지 않고 그립니다`, `시퀀스는 1단계 런타임에서 셋업 프레임으로 그립니다`, `이미지를 찾을 수 없습니다: {경로}`, `이미지 폴더가 프로젝트 밖입니다: {경로}`, `묶은 텍스처가 {max}px를 넘어 줄였습니다`. `ctx.LogImportWarning`으로 내고 에셋 `warnings`에도 담아 인스펙터(`KGBoneSkeletonDataEditor`)에 표시.

- [ ] **Step 1: 실패하는 테스트(유니티 EditMode)** `m10_import_creates_data_texture_materials`: 테스트가 임시 폴더 `Assets/KGBoneTestTmp/`에 `Tests/Golden/unity-stage1.kgbone`과 그 이미지를 복사하고 `AssetDatabase.ImportAsset` → 메인 에셋이 `KGBoneSkeletonData`, `Texture != null`, 블렌드 4종 머티리얼 모두 있음, `Model.Bones.Count` 일치, `Warnings` 비어 있음, JSON에 `"editor"` 없음. **Review Focus 1** `m10_missing_and_korean_subfolder_images`: 이미지 하나를 지우고 하나를 `팔/왼쪽.png`로 둔 파일 → 경고에 `이미지를 찾을 수 없습니다: …` 1건, `팔/왼쪽`은 `TryGetImage` 성공. `m10_unsupported_features_warn`: `samples/kgbone/full-4.3.kgbone` → 경고에 `제약 4개는` 포함. 테스트 끝에 임시 폴더 삭제.
- [ ] **Step 2:** 작성 → Task 12의 Sandbox가 아직 없으므로 이 작업에서 `unity/KGBoneSandbox/`의 최소 프로젝트(Packages/manifest.json에 `"com.kgbone.runtime": "file:../../com.kgbone.runtime"`, `"testables": ["com.kgbone.runtime"]`, URP)를 먼저 만들고, 아래로 EditMode 테스트 실행:
  `"C:/Program Files/Unity/Hub/Editor/6000.3.17f1/Editor/Unity.exe" -batchmode -nographics -projectPath unity/KGBoneSandbox -runTests -testPlatform EditMode -testResults unity/KGBoneSandbox/Logs/editmode.xml -logFile -`
  (같은 프로젝트를 에디터로 열어 둔 상태면 실패하므로 닫는다. Unity MCP가 연결돼 있으면 `run_tests`를 써도 된다.) → 통과.
- [ ] **Step 3:** 커밋 `[ADD] 유니티 런타임 데이터 에셋과 .kgbone 가져오기 도구 추가` 후 push.

### Task 11: 셰이더, 메시 만들기, 재생 컴포넌트

**Files:** Create `Shaders/KGBoneUnlit.shader`, `Runtime/MeshBuilder.cs`, `Runtime/KGBoneAnimation.cs`, `Runtime/KGBoneEvent.cs`, `Tests/Editor/AnimationComponentTests.cs`

**Interfaces:**
- Produces: `public readonly struct KGBoneEvent { public string Name; public float Time; public int Int; public float Float; public string String; public float Volume; public float Balance; }`
- Produces: `[ExecuteAlways, RequireComponent(typeof(MeshFilter), typeof(MeshRenderer))] public sealed class KGBoneAnimation : MonoBehaviour { public KGBoneSkeletonData skeletonData; public string skin; public string animationName; public bool loop = true; public float timeScale = 1; public bool playOnAwake = true; public bool flipX, flipY; public string sortingLayerName = "Default"; public int sortingOrder; /* MeshRenderer는 인스펙터에 Sorting Layer를 보여 주지 않으므로 컴포넌트가 노출하고 OnValidate·Awake에서 렌더러에 넣는다 */ public void Play(string animation, bool loop); public void Stop(); public float Time { get; set; } public bool IsPlaying; public string CurrentAnimation; public event Action<KGBoneEvent> EventFired; public event Action Completed; public void Advance(float dt); /* Update가 Time.deltaTime으로 부름. 테스트는 직접 부름 */ public bool TryGetBone(string name, out Vector3 position, out float rotation, out Vector2 scale); public PoseResult LastPose; }`
- Produces: `internal static class MeshBuilder { public static void Build(PoseResult pose, KGBoneSkeletonData data, bool flipX, bool flipY, Mesh mesh, List<Material> materialsOut); }` — 정점 = positions ÷ pixelsPerUnit(뒤집기면 부호 반전), uv = 묶은 텍스처 사각형 안으로 옮기고 v 뒤집기(`v' = rect.yMax − v × rect.height`), 정점 색 = Color, 어두운 색 = `uv2`(rgb, 없으면 0), 연속 같은 블렌드끼리 서브메시. 이미지를 못 찾은 항목은 건너뛴다. 메시·리스트는 다시 쓴다.
- 셰이더 `KGBone/Unlit`: `Tags { "RenderPipeline"="UniversalPipeline" "Queue"="Transparent" "RenderType"="Transparent" }`, `Blend [_SrcBlend] [_DstBlend]`, `ZWrite Off`, `Cull Off`, 색 공식은 Global Constraints.
- 시간: `Advance(dt)`에서 `Time += dt × timeScale`. 반복이 아니면 `Time`을 길이에서 멈추고 처음 도달할 때만 `Completed`. 이벤트는 `LastTime = 이전 Time`으로 `SkeletonPoser.Compute`에 넘겨 받는다(반복이면 실제 누적 시간을 그대로 넘긴다 — 핵심이 바퀴 계산을 한다).
- `Play(이름)`: 없는 이름이면 `Debug.LogWarning("애니메이션을 찾을 수 없습니다: {이름}")` 후 상태 유지. `Play(null)`/`Stop()` = 셋업 포즈, 재생 멈춤.
- 본 읽기: `PoseResult.Bones[i]`의 위치 ÷ pixelsPerUnit(뒤집기 반영) → `transform.TransformPoint`; 회전 = `atan2(c, a)`도(뒤집기 반영) + transform 회전; 크기 = (√(a²+c²), √(b²+d²)).

- [ ] **Step 1: 실패하는 테스트(EditMode)** `m11_play_advances_and_builds_mesh`: Task 10의 임시 가져오기로 만든 데이터, `Play("걷기", true)`, `Advance(0.5f)` → `LastPose`가 같은 요청의 `SkeletonPoser.Compute`와 같고 메시 정점 수 = 그리기 항목 정점 합, 서브메시 수 = 연속 블렌드 묶음 수. `m11_events_fired_once_per_key`: 0→1.2초를 0.1초씩 → 이벤트 이름 순서가 정답(키 0, …, 1.0=0)과 같음. **Review Focus 2** `m11_unknown_animation_keeps_state`(LogAssert.Expect 경고, CurrentAnimation 그대로), **Review Focus 3** `m11_non_loop_holds_last_frame_and_completes_once`(길이 1, `Advance(0.7)` 두 번 → Time 1, Completed 1회, 다시 `Advance` 해도 1회), **Review Focus 6** `m11_no_data_draws_nothing`(skeletonData null로 `Advance` → 예외 없음, 메시 정점 0). `m12_try_get_bone_with_flip`: flipX이면 본 x 위치의 부호가 반대.
- [ ] **Step 2:** EditMode 실행(Task 10 Step 2 명령) 실패 확인 → 작성 → 통과.
- [ ] **Step 3:** 커밋 `[ADD] 유니티 런타임 재생 컴포넌트와 URP 셰이더 추가` 후 push.

### Task 12: 본 따라가기, 확인용 프로젝트, 눈으로 확인

**Files:** Create `Runtime/KGBoneBoneFollower.cs`, `tools/unity-sandbox.mjs`, `unity/KGBoneSandbox/Assets/Scenes/Sandbox.unity`(골렘 등 자리 비운 기본 장면); Modify 루트 `package.json`, `unity/com.kgbone.runtime/package.json`(URP 버전 확정)

**Interfaces:**
- Produces: `[ExecuteAlways] public sealed class KGBoneBoneFollower : MonoBehaviour { public KGBoneAnimation target; public string boneName; public bool followRotation; }` — `LateUpdate`에서 `target.TryGetBone`.
- Produces: 루트 스크립트 `"unity:sandbox": "node tools/unity-sandbox.mjs"` — `samples/kgbone/*.kgbone`(제약 없는 것)과 `samples/images/`를 `unity/KGBoneSandbox/Assets/Samples/`로 복사(이미지 폴더 상대 경로가 맞게 `Samples/images/`).

- [ ] **Step 1:** Sandbox에서 Package Manager가 받은 URP 버전을 확인하고, 패키지 `package.json`의 URP 의존성을 Unity 6000.0이 받을 수 있는 가장 낮은 17.0.x로 확정(`Packages/packages-lock.json`의 해결 결과로 확인).
- [ ] **Step 2: 실패하는 테스트(EditMode)** `m12_follower_tracks_bone`: 본을 따라가는 GameObject의 위치가 `TryGetBone` 결과와 같음(1e-4).
- [ ] **Step 3:** 작성 → 통과 → `npm run unity:sandbox` → 에디터로 Sandbox를 열어 `unity-stage1.kgbone`을 장면에 놓고 재생, 블렌드·어두운 색·그리기 순서·스킨이 KGBone 웹 판과 같게 보이는지 눈으로 비교(스크린샷을 `test-results/`에 남김, 커밋 안 함). 사용자의 `local/golem/`을 `Assets/Samples/golem/`에 복사해 idle 재생도 확인.
- [ ] **Step 4:** 커밋 `[ADD] 유니티 런타임 본 따라가기와 확인용 프로젝트 추가` 후 push.

### Task 13: 문서와 규칙

**Files:** Modify `docs/SPEC.md`, `CLAUDE.md`, `docs/GAPS.md`, `README.md`, `docs/HANDOFF.md`; Create `unity/com.kgbone.runtime/README.md`, `CHANGELOG.md`

- [ ] **Step 1:** SPEC.md에 "M. 유니티 런타임" 절: M01 패키지 구조·설치, M02 JSON·.kgbone 읽기, M03 본 월드(상속 5종), M04 곡선, M05 본 키, M06 슬롯 키·그리기 순서, M07 그리기 목록(정답 비교), M08 스킨·활성, M09 이벤트·반복 시점, M10 가져오기(텍스처·경고), M11 재생 컴포넌트, M12 본 읽기·따라가기. 각 줄의 완료 기준에 위 테스트 이름을 적는다.
- [ ] **Step 2:** CLAUDE.md: 2장 폴더 구조에 `unity/`, 1장 또는 3장에 예외 문구 "유니티 런타임(`unity/com.kgbone.runtime/Runtime/Core`)은 계산을 C#으로 다시 구현한다. 결과는 `npm run unity:golden`이 만든 정답과 `npm run unity:test`로 비교해 같음을 보장한다", 5장 명령(`unity:golden`, `unity:test`, `unity:sandbox`, 유니티 EditMode 실행 명령), 6장 완료 기준에 `npm run unity:test` 추가.
- [ ] **Step 3:** GAPS.md에 1단계 비목표(제약·클리핑·시퀀스·섞기·스킨 바꾸기·본 덮어쓰기·Animator 연동)와, Task 1에서 `dotnet test`가 막혔다면 그 사실. README와 패키지 README: 설치 주소(태그 없이 `#main`, 라이선스 결정 전임을 명시), 가져오기·컴포넌트·본 따라가기 사용법, 지원·미지원 표. HANDOFF 1-1을 진행 상황으로 갱신.
- [ ] **Step 4:** `npm run test:ui`, `npm run test:mcp`, `npm run unity:test`(또는 CI), 유니티 EditMode 전부 통과 확인 → 커밋 `[CHG] 유니티 런타임 1단계 명세·규칙·사용법 문서 갱신` 후 push.
