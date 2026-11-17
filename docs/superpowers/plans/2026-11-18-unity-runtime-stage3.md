# KGBone 유니티 런타임 3단계 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 유니티 런타임이 시퀀스·클리핑을 핵심과 같게 그리고, 게임 코드가 번호 트랙(섞기·예약·비우기), 첨부물 바꾸기, 스킨 합치기, 본 덮어쓰기로 재생을 조작할 수 있게 한다.

**Architecture:** 시퀀스·클리핑은 핵심 `anim/attachment_keys.rs`, `mesh/triangulate.rs`, `pose/clipping.rs`, `pose/draw.rs`를 C# 9로 줄 단위 대응시켜 옮기고 정답으로 비교한다. 트랙·덮어쓰기·스킨 합치기는 핵심에 없으므로 계산부(`Runtime/Core`)에 새 클래스로 두고 dotnet 단위 테스트로 확인하며, 컴포넌트 `KGBoneAnimation`은 얇게 감싼다. 세 묶음(①시퀀스·클리핑 ②트랙 ③첨부물·스킨·본 덮어쓰기), 묶음마다 push + CI 확인.

**Tech Stack:** C# 9(유니티 호환), .NET 10 + NUnit(`npm run unity:test`), Unity 6000.3 + URP(EditMode), Node 24(정답 생성기).

**Spec:** `docs/superpowers/specs/2026-11-18-unity-runtime-stage3-design.md` (앞 단계 계획: `docs/superpowers/plans/2026-11-17-unity-runtime-stage2.md`)

## Global Constraints

- 클린룸: 근거는 `docs/FORMAT.md`, `crates/kgbone-core` 코드, 이 설계 문서뿐. `EsotericSoftware/spine-runtimes`의 어떤 파일도 열지 않는다. 이름에 "Spine"을 쓰지 않는다. 트랙 API는 설계 5.3 정의만 따른다.
- 계산부 `Runtime/Core`: UnityEngine 없음, 외부 의존성 0개, **C# 9 문법만**, `float` + `MathF`(핵심과 같게).
- 핵심(Rust) 계산은 바꾸지 않는다. 핵심 버그로 보이면 C#은 핵심과 같게 옮기고 GAPS.md에 적어 보고.
- 정답 비교 오차 1e-3. 기존 정답 912개와 기존 테스트(dotnet 251, EditMode 276)는 모든 Task에서 그대로 통과(설계 목표 6: 새 API를 안 쓰면 2단계와 같음).
- `defaultMix` 기본 0. 새 기능(트랙·스킨 합치기·덮어쓰기)은 플레이 중 상태(직렬화 안 함), 인스펙터 새 필드는 `defaultMix` 하나.
- 매 프레임 새 할당을 늘리지 않는다(포즈 버퍼·목록 재사용).
- 테스트 이름에 SPEC 번호: `m17`(시퀀스), `m18`(클리핑), `m19`(트랙 계산부), `m20`(트랙 컴포넌트·이벤트), `m21`(첨부물·스킨 합치기), `m22`(본 덮어쓰기). Task 3·6·8에서 SPEC.md M절에 적는다.
- 새 C# 테스트 파일은 `unity/tests/KGBone.Core.Tests/KGBone.Core.Tests.csproj`의 Compile 목록에도 넣는다(EditMode 전용 테스트는 제외). 새 `.cs`·정답 파일에는 `.meta`를 붙인다(2단계 규칙: .cs는 최소 형식 + 새 guid, 정답 json/kgbone은 기존 정답 meta의 importer 블록 복사).
- 커밋: `[ADD]/[CHG]/[DEL]/[FIX]` + 한국어, 공동 작업자 줄 금지. 묶음 끝(Task 3, 6, 8)마다 `git push origin HEAD:main`(강제 금지) 후 GitHub Actions 3개 워크플로 성공 확인. `.claude/`, `local/`, 골렘 파일은 커밋하지 않는다.
- 묶음 끝마다 `npm run unity:test`, `npm run unity:golden` 후 `git status`(정답이 최신인지), `npm run unity:editmode`(KGBoneSandbox, 에디터로 열려 있지 않을 때)를 돌린다. 이 PC에는 cargo가 없다: 정답 생성은 `npm run wasm:fetch`로 받은 kgbone.wasm을 쓴다(핵심을 안 바꾸므로 같음).

## Review Focus

1. **없는 이름**: `SetAnimation`/`AddAnimation`에 없는 애니메이션, `SetAttachment`에 없는 슬롯·첨부물, 덮어쓰기에 없는 본, `SetSkins`에 없는 스킨 → 예외 없이 경고 로그 한 번(같은 이름 반복은 다시 안 냄), 트랙·상태는 그대로. → Task 6 `m20_unknown_animation_keeps_track`, Task 8 `m21_unknown_names_warn_once`.
2. **섞는 시간 > 애니메이션 길이, 또는 섞는 중 이전 항목(반복 아님)이 끝남**: 이전 항목은 끝 프레임을 유지한 채 섞이고, 섞기가 끝날 때까지 튀지 않는다. → Task 4 `m19_mix_longer_than_from_holds_last_frame`.
3. **시퀀스 프레임 이미지 일부가 없음**: 가져올 때 경고, 실행 중 그 프레임은 그리지 않고(없는 이미지와 같은 처리) 예외 없음. → Task 1 `m17_missing_frame_image_skipped`(EditMode).
4. **클립 다각형 점 3개 미만 / 넓이 0 / 자를 결과가 비어 있음**: 핵심과 같이 자르지 않음(점 3개 미만) 또는 그 항목을 그리지 않음, 예외 없음. → Task 2 `m18_degenerate_polygon_no_clip`.
5. **`ClearTrack` 뒤 바로 같은 트랙 `SetAnimation`**(빠지는 중 새 항목): 얼린 포즈에서 섞여 들어오고 튐 없음. → Task 4 `m19_set_during_clear_no_jump`.

---

## 파일 구조

```
unity/com.kgbone.runtime/
  Runtime/Core/Model/
    AnimationData.cs        (고침) SequenceTimeline 목록
    AttachmentData.cs       (고침) ClippingAttachmentData
    KGBoneLoader.cs         (고침) sequence 타임라인 저장, clipping 첨부물, ImagePaths 모든 프레임
    SkeletonModel.cs        (고침) HasClipping/HasSequence 제거(경고 없앰)
  Runtime/Core/Anim/
    AnimationApplier.cs     (고침) 시퀀스 키 적용, skin → SkinSet
    SequenceKeys.cs         (새) sequence_frame 대응
  Runtime/Core/Mesh/
    Triangulator.cs         (새) mesh/triangulate.rs 대응
  Runtime/Core/Pose/
    Clipper.cs              (새) pose/clipping.rs 대응
    DrawListBuilder.cs      (고침) 시퀀스 경로, 자르기
    PoseState.cs            (고침) SequenceFrames, CopyFrom, 키 표시(Keyed)
    PoseBlend.cs            (새) 두 포즈 섞기
    AnimationTracks.cs      (새) 트랙·항목·예약·섞기·이벤트
    SkinSet.cs              (새) 켠 스킨 목록(뒤 스킨 우선)
    PoseOverrides.cs        (새) 첨부물·본 덮어쓰기 값
    SkeletonPoser.cs        (고침) 트랙·덮어쓰기·스킨 목록 경로
    PoseResult.cs           (고침) PoseRequest.Tracks/Skins/Overrides, FiredEvent.Track/Animation
  Runtime/KGBoneAnimation.cs (고침) 트랙·첨부물·스킨·본 API
  Runtime/KGBoneEvent.cs    (고침) Track, Animation
  Editor/KGBoneImporter.cs  (고침) 클리핑·시퀀스 경고 제거
  Tests/Editor/
    Stage3UnitTests.cs      (새, dotnet에도 포함) m17·m18·m19·m21·m22 계산부
    AnimationComponentTests.cs, ImporterTests.cs (고침) m17·m20·m21·m22 EditMode
  Tests/Golden/             (다시 생성)
tools/gen-samples.mjs       (고침) unity-stage3-sequence, unity-stage3-clipping (+ 시퀀스 프레임 이미지)
tools/unity-golden.mjs      (고침) 대상 추가
samples/kgbone/unity-stage3-*.kgbone, samples/images/… (생성물)
```

---

## 묶음 ① 시퀀스 · 클리핑

### Task 1: 시퀀스

**Files:** Create `Anim/SequenceKeys.cs`, `Tests/Editor/Stage3UnitTests.cs`; Modify `AnimationData.cs`, `KGBoneLoader.cs`, `AnimationApplier.cs`, `PoseState.cs`, `DrawListBuilder.cs`, `AttachmentData.cs`, `SkeletonModel.cs`, `Editor/KGBoneImporter.cs`, `tools/gen-samples.mjs`, `tools/unity-golden.mjs`, csproj; Test `Stage3UnitTests.cs`, `AnimationComponentTests.cs`, `ImporterTests.cs`

**Interfaces:**
- Produces: `sealed class SequenceTimeline { string Skin, Slot, Attachment; Key[] Keys; }`, `AnimationData.Sequences : List<SequenceTimeline>`(기존처럼 길이에 키 시간 포함).
- Produces: `static class SequenceKeys { static int? Frame(Key[] keys, float t, int count) }` — 핵심 `sequence_frame`과 같은 규칙(계단 키 찾기는 기존 `Curve`의 계단 인덱스 함수를 쓴다; `v[0]` 시작, `v[1]` 간격, `S` 모드 문자열, `+1e-4` 후 floor, 결과 `[0, count−1]`로 자름).
- Produces: `PoseState.SequenceFrames : Dictionary<(string slot, string attachment), int>`(셋업 때 비움), `AttachmentData.ImagePath(string key, int? frame)` — 핵심 `sequence_path`: `max(frame ?? Setup, 0) + Start`을 `Digits` 자리 0 채움. 기존 `ImagePath(key)`는 `ImagePath(key, null)`.
- 시퀀스 키의 스킨 범위는 deform과 같다(기본 스킨 + 켠 스킨). 섞기 비율이 있을 때는 이산 값(`Mix.Discrete`).

- [ ] **Step 1: 시험 파일.** `tools/gen-samples.mjs`에 `unity-stage3-sequence.kgbone`을 더한다: region 시퀀스 슬롯 7개(모드 7종 각 하나, count 4, delay 0.1), delay 0 키 하나, digits 0과 2를 쓰는 첨부물, 키 없는 시퀀스 첨부물(셋업 프레임 1), 메시 시퀀스 하나. 프레임 이미지(`samples/images/` 아래 새 PNG, 기존 생성기의 PNG 쓰기 함수 사용)도 만든다. `npm run samples` 후 `tools/unity-golden.mjs` TARGETS에 `["unity-stage3-sequence", "samples/kgbone/unity-stage3-sequence.kgbone"]`. 각 키 시간 사이·경계(키 시간, 키 시간 + delay×k)를 지나는 시점이 정답 시점에 들어가는지 확인하고 모자라면 그 대상의 시점 목록을 늘린다. `npm run unity:golden`.
- [ ] **Step 2: 실패하는 테스트.** `npm run unity:test` → 새 정답의 `image`가 셋업 프레임 경로라 FAIL. `Stage3UnitTests.cs`에:
  - `m17_sequence_frame_modes` — count 4, start 0, delay 0.1에서 t=0.25: hold→0, once→2, loop→2, pingpong→2, onceReverse→1, loopReverse→1, pingpongReverse→1; t=0.55: once→3, loop→1, pingpong→1(p=5, n2=6 → q=5 ≥4 → 1).
  - `m17_sequence_delay_zero_holds_start` — delay 0이면 start 고정.
  - `m17_image_path_digits` — Start 1, Digits 2, frame 3 → `"base04"`; Digits 0 → `"base4"`.
  - `m17_image_paths_include_all_frames` — count 4, start 1 → `ImagePaths`가 base1~base4를 낸다.
- [ ] **Step 3: 구현.** 불러오기(지금 버리는 `sequence` 타임라인을 저장, 키 `S` 읽기 포함 — 기존 `ReadKeys`가 `s`를 읽는지 확인하고 없으면 더함), 적용, 그리기 경로, `ImagePaths`가 시퀀스 첨부물의 모든 프레임을 냄. `HasSequence`와 가져오기 경고 "시퀀스는 …"을 지운다.
- [ ] **Step 4:** `npm run unity:test` → 전부 PASS(기존 + 새 정답 + m17).
- [ ] **Step 5: EditMode.** `AnimationComponentTests`/`ImporterTests`에 `m17_import_packs_all_sequence_frames`(묶은 텍스처 항목에 프레임 4개, 경고 없음), `m17_missing_frame_image_skipped`(프레임 하나 지운 임시 파일: 가져오기 경고 "이미지를 찾을 수 없습니다", 그 프레임 시점에 예외 없이 그 항목만 빠짐). 기존 시퀀스 경고 기대 테스트가 있으면 고친다. `npm run unity:editmode` → PASS.
- [ ] **Step 6: 커밋** `[ADD] 유니티 런타임: 시퀀스 프레임 키와 모든 프레임 이미지 묶기, 정답 비교`.

### Task 2: 삼각분할과 클리퍼

**Files:** Create `Runtime/Core/Mesh/Triangulator.cs`, `Runtime/Core/Pose/Clipper.cs`; Test `Stage3UnitTests.cs`

**Interfaces:**
- Produces: `static class Triangulator { static int[] Triangulate(IReadOnlyList<float> polygon) }` — `mesh/triangulate.rs`의 `triangulate` 줄 단위 대응.
- Produces: `sealed class Clipper { static Clipper Create(IReadOnlyList<float> polygon, bool convex, bool inverse) /* 점 3개 미만 null */; void Clip(List<float> positions, List<float> uvs, List<int> indices) /* 제자리에서 바꿈, 내부 임시 목록 재사용 */ }` — `pose/clipping.rs`(convex_hull, clip_half, intersect_convex, subtract_convex, 부채꼴 삼각형화, 무게중심 uv, AREA_EPS 1e-6) 대응.

- [ ] **Step 1: 실패하는 테스트**(`Stage3UnitTests.cs`):
  - `m18_triangulate_concave` — L자 6각형 → 삼각형 4개, 넓이 합 = 다각형 넓이(1e-4).
  - `m18_clip_square_by_concave_polygon` — 핵심 `clipping.rs` 테스트 `clip_square_by_concave_polygon`과 같은 입력·기대값(그 파일 아래 테스트를 읽어 같은 숫자로).
  - `m18_inverse_removes_inside` — 10×10 사각형을 가운데 4×4로 inverse 자르기 → 남은 넓이 84.
  - `m18_convex_uses_hull` — 오목 클립 + convex=true → 결과 넓이 = 껍질과의 교집합 넓이.
  - `m18_uv_barycentric` — 자른 점의 uv가 원래 삼각형의 무게중심 보간과 같음.
  - `m18_degenerate_polygon_no_clip` — 점 2개 → `Create`가 null; 넓이 0 클립 → 결과 비어 있음, 예외 없음.
- [ ] **Step 2:** `npm run unity:test` → 컴파일 실패(타입 없음).
- [ ] **Step 3: 구현**(핵심과 같은 순서·float).
- [ ] **Step 4:** `npm run unity:test` → PASS.
- [ ] **Step 5: 커밋** `[ADD] 유니티 런타임: 삼각분할과 클리핑 다각형 자르기`.

### Task 3: 그리기 목록 클리핑, 정답 비교, 묶음 ① 문서와 push

**Files:** Modify `AttachmentData.cs`, `KGBoneLoader.cs`, `DrawListBuilder.cs`, `SkeletonModel.cs`, `Editor/KGBoneImporter.cs`, `tools/gen-samples.mjs`, `tools/unity-golden.mjs`, docs; Test `ImporterTests.cs`

**Interfaces:**
- Consumes: Task 2 `Clipper`.
- Produces: `sealed class ClippingAttachmentData : AttachmentData { string End; int VertexCount; bool Weighted, Inverse, Convex; float[] Vertices; ImagePath → null }`(`OtherAttachment` "clipping" 대신).
- `DrawListBuilder.Build`: 핵심 `draw.rs`와 같이 clipping 첨부물 슬롯에서 `Clipper.Create(월드 정점(가중치·deform 반영, 기존 path용 정점 함수 재사용), Convex, Inverse)`, 이후 항목을 `Clip`, 결과 인덱스 0이면 그 항목 버림, `End`와 같은 이름 슬롯을 지난 뒤 자르기 끝(첨부물이 없어도 슬롯 이름으로 끝남 — 핵심과 같음), 클리핑 슬롯 자체는 그리지 않음.

- [ ] **Step 1: 시험 파일·정답.** `gen-samples.mjs`에 `unity-stage3-clipping.kgbone`: 보통·inverse·convex 클리핑, 오목 클립 다각형, 가중치 메시 + 가중치 클립 다각형, end 지정·미지정(끝까지), clipping 정점 deform 키 하나. TARGETS에 `["spine43-clipping", "samples/spine43/clipping.json"]`, `["unity-stage3-clipping", "samples/kgbone/unity-stage3-clipping.kgbone"]`. `npm run samples && npm run unity:golden`.
- [ ] **Step 2:** `npm run unity:test` → 새 정답 FAIL(자르지 않음).
- [ ] **Step 3: 구현.** `HasClipping`과 경고 "클리핑은 …" 제거. `ImporterTests`에서 full-4.3 등 클리핑 파일이 클리핑 경고 없음을 확인하는 단언 `m18_importer_no_clipping_warning`.
- [ ] **Step 4:** `npm run unity:test`(전부 PASS), `npm run unity:editmode`(PASS).
- [ ] **Step 5: 문서.** SPEC M절에 "유니티 런타임 3단계" 표 시작: M17(시퀀스), M18(클리핑) 완료 기준과 테스트 이름. 패키지 README 지원 표에서 클리핑·시퀀스를 지원으로, CHANGELOG "0.3.0(개발 중)"에 추가, GAPS G-26에서 클리핑·시퀀스 문장 갱신, HANDOFF 1-1에 3단계 진행 한 줄.
- [ ] **Step 6: 커밋·push.** `[ADD] 유니티 런타임: 그리기 목록 클리핑과 정답 비교, 3단계 묶음 ① 문서`. `git push origin HEAD:main`, CI 3개 성공 확인.

## 묶음 ② 트랙과 섞기

### Task 4: 포즈 섞기와 트랙 계산부

**Files:** Create `Pose/PoseBlend.cs`, `Pose/AnimationTracks.cs`; Modify `PoseState.cs`, `AnimationApplier.cs`; Test `Stage3UnitTests.cs`

**Interfaces:**
- Produces: `PoseState.CopyFrom(PoseState other)`(배열 값 복사, Deform·SequenceFrames 사전은 키별 복사·버퍼 재사용).
- Produces: `static class PoseBlend { static void Lerp(SkeletonModel m, PoseState x, PoseState y, float t, PoseState output) }` — 로컬 본 값 선형(회전·shear는 `Mix.BlendAngle`과 같은 짧은 쪽 규칙), 슬롯 색·어두운 색 선형(어두운 색 한쪽만 있으면 t≥0.5 쪽), 첨부물·그리기 순서·시퀀스 프레임·Inherit은 t≥0.5면 y, deform은 같은 키끼리 선형(한쪽만 있으면 없는 쪽을 셋업 정점(0 오프셋)으로 보고 선형), 제약 값 배열 선형(정수형 bend 등 이산 값은 t≥0.5면 y — 2단계 `ConstraintKeys`가 이산으로 다루는 값과 같게).
- Produces(공개, `KGBone.Core`):
  ```csharp
  public sealed class TrackEntry { public string Name; public bool Loop; public float Time; public float MixDuration; public float Delay; internal AnimationData Data; internal float LastTime; internal float MixTime; }
  public readonly struct TrackCompletion { public readonly int Track; public readonly string Name; }
  public sealed class AnimationTracks
  {
      public AnimationTracks(SkeletonModel model);
      public TrackEntry SetAnimation(int track, string name, bool loop, float mix);   // 없는 이름이면 null, 트랙 그대로
      public TrackEntry AddAnimation(int track, string name, bool loop, float delay, float mix);
      public void ClearTrack(int track, float mix);       // mix 0이면 즉시
      public void AddEmpty(int track, float delay, float mix);
      public void ClearAll();                              // 모든 트랙 즉시 비움
      public void SetAlpha(int track, float alpha);        // 0~1로 자름
      public TrackEntry Current(int track);                // 없으면 null
      public int TrackCount { get; }
      public void Update(float dt, List<TrackCompletion> completed);  // dt는 timeScale 곱한 값
      internal void Apply(SkeletonModel m, SkinSet skins, PoseState pose, TrackScratch scratch);
      internal void CollectEvents(SkeletonModel m, List<FiredEvent> output); // 지금 항목만, [LastTime, Time), 반복 아닌 항목이 이번 Update에 끝에 닿았으면 끝 시간 키도(기존 컴포넌트 규칙 G-27)
      internal void ResetKeysHit(SkeletonModel m, PhysicsState physics);     // 지금 항목들의 physics reset 키 구간
  }
  internal sealed class TrackScratch { /* X, Y 공용 PoseState 둘 + 트랙별 얼린 포즈(필요할 때 만듦) */ }
  ```
  `SkinSet`은 Task 7 전까지 이 Task에서 `sealed class SkinSet { public static SkinSet Single(string skin); }`만 만들어 둔다(Task 7이 채움). `AnimationApplier.Apply`의 `string skin` 인자는 이 Task에서는 그대로 두고, `Apply`가 `skins` 대신 `SkinSet.Primary`(첫 이름)를 넘긴다 — Task 7에서 바꾼다.
- 섞기 규칙은 설계 5.3 그대로: R = 트랙 n 적용 전 포즈, X = R + 이전 항목(비율 Alpha) 또는 얼린 포즈 또는 R, Y = R + 지금 항목(비율 Alpha) 또는 R(비우는 중), 결과 = `PoseBlend.Lerp(X, Y, t)`. 섞는 중이 아니면 R에 지금 항목만 `Apply`(2단계와 같은 경로).
- 윗 트랙 얼린 포즈는 "그 트랙 키가 있던 값"만: `PoseState`에 이번 적용에서 키가 닿은 값 표시(`Keyed` 비트 배열: 본 채널·슬롯·deform·제약)를 두고 얼릴 때 표시된 값만 복사, 나머지는 R을 따른다(설계 판단을 이렇게 구현).
- 시간: `Time += dt`. 반복 아닌 항목은 `Duration`에서 멈추고 그 Update에 `TrackCompletion` 한 번. 예약은 지금 항목의 (반복이면 이번 바퀴) 끝 + `Delay`에 시작하고 남은 시간을 새 항목 시간으로 넘긴다. 섞기 끝나면 이전 항목·얼린 포즈 버림. 반복 항목의 `AnimationTime`은 기존 `AnimationApplier.AnimationTime`.

- [ ] **Step 1: 실패하는 테스트**(본 2개 + 애니메이션 "걷기"(두 본 회전 키 0→90), "달리기"(두 본 회전 키 0→−90), "팔"(본 1만 키) 을 코드로 만든 작은 모델):
  - `m19_single_track_matches_plain_apply` — 트랙 0에 걷기만, 여러 시점에서 `AnimationApplier.Apply` 직접 결과와 비트 단위 같음.
  - `m19_mix_midpoint_is_between_poses` — 걷기 t=0.5(45°) → 달리기(mix 0.2) 0.1초 뒤: 두 본 회전 = 두 포즈 값의 중간(셋업 0으로 꺼지지 않음, 짧은 쪽).
  - `m19_mix_end_no_jump` — 섞기 끝 프레임 전후 회전 차이가 프레임 움직임보다 크지 않음; 이전 항목에만 키가 있는 값도 마찬가지.
  - `m19_upper_track_only_keyed_values` — 트랙 0 걷기 + 트랙 1 팔: 본 2는 걷기 값 그대로, 본 1은 팔 값.
  - `m19_queue_starts_at_end_plus_delay` — 반복 아닌 1초 + `AddAnimation(delay 0.25)`: 1.25초에 새 항목 시간 0, 넘친 시간은 새 항목으로.
  - `m19_clear_track_fades_out` — 트랙 1 `ClearTrack(0.2)`: 0.1초 뒤 본 1 = 팔과 걷기의 중간, 0.2초 뒤 걷기 값, `Current(1)` null.
  - `m19_completion_once` — 반복 아님 끝에서 `TrackCompletion` 정확히 한 번.
  - `m19_mix_longer_than_from_holds_last_frame`(Review Focus 2).
  - `m19_set_during_clear_no_jump`(Review Focus 5), `m19_set_during_mix_freezes`(섞는 중 또 바꿈: 바꾼 프레임 전후 튐 없음).
  - `m19_alpha_half` — `SetAlpha(1, 0.5)`면 본 1 = 걷기와 팔의 중간.
  - `m19_events_from_current_only` — 섞이며 빠지는 항목의 이벤트는 안 남, 지금 항목 이벤트에 Track 번호.
- [ ] **Step 2:** `npm run unity:test` → 컴파일 실패.
- [ ] **Step 3: 구현.**
- [ ] **Step 4:** `npm run unity:test` → 전부 PASS.
- [ ] **Step 5: 커밋** `[ADD] 유니티 런타임: 포즈 섞기와 번호 트랙 계산부`.

### Task 5: 포즈 계산기에 트랙 연결

**Files:** Modify `PoseResult.cs`, `SkeletonPoser.cs`, `EventFinder.cs`(FiredEvent 필드); Test `Stage3UnitTests.cs`

**Interfaces:**
- Produces: `PoseRequest.Tracks : AnimationTracks`(기본 null). null이면 지금 `Animation`/`Time`/`Looping` 경로 그대로(정답 비교 경로). null이 아니면 `Animation`·`Time`·`Looping`은 쓰지 않고 트랙을 적용하고, 이벤트는 `LastTime`이 있을 때(값은 쓰지 않음, "이벤트를 낼지" 표시로만) `CollectEvents`로, physics reset은 `ResetKeysHit`로.
- Produces: `FiredEvent.Track : int`, `FiredEvent.Animation : string`(기존 경로는 0, 요청 애니메이션 이름).

- [ ] **Step 1: 실패하는 테스트** — `m19_poser_tracks_equal_plain_request`: 정답 파일 `unity-stage1`의 애니메이션 하나를 트랙 0 하나로 돌린 결과(본·그리기·이벤트)가 `Animation` 요청 결과와 같음(여러 dt). `m19_poser_two_tracks_events_tagged`.
- [ ] **Step 2:** FAIL 확인. **Step 3:** 구현. **Step 4:** `npm run unity:test` PASS(정답 912개 포함).
- [ ] **Step 5: 커밋** `[ADD] 유니티 런타임: 포즈 계산에 트랙 적용과 트랙별 이벤트`.

### Task 6: 컴포넌트 트랙 API, 묶음 ② 문서와 push

**Files:** Modify `Runtime/KGBoneAnimation.cs`, `Runtime/KGBoneEvent.cs`, docs; Test `AnimationComponentTests.cs`

**Interfaces (공개, 이름 확정 — 설계 5.3):**
```csharp
public float defaultMix;                       // 인스펙터, 기본 0
public event Action<int, string> TrackCompleted;
public TrackEntry SetAnimation(int track, string name, bool loop, float mix = -1);
public TrackEntry AddAnimation(int track, string name, bool loop, float delay = 0, float mix = -1);
public void ClearTrack(int track, float mix = -1);
public void AddEmpty(int track, float delay = 0, float mix = -1);
public void SetTrackAlpha(int track, float alpha);
public string GetTrackAnimation(int track);
```
`mix < 0` → `defaultMix`. `KGBoneEvent`에 `public readonly int Track; public readonly string Animation;` + 새 생성자(기존 생성자는 Track 0, Animation null로 남김).
- 컴포넌트는 `AnimationTracks`를 하나 들고 플레이·에디터 모드 모두 그것으로 계산한다. 에디터 모드에서는 인스펙터 `animationName` 0초만(지금과 같은 화면).
- `Play(name, loop)` = `SetAnimation(0, name, loop, 0)`(즉시, 지금과 같음). `Stop()` = `ClearAll()` + 셋업. 실행 중 `animationName` 필드가 바뀐 것을 감지하면 `SetAnimation(0, …, defaultMix)`.
- `Time`(get/set), `CurrentAnimation`, `IsPlaying`은 트랙 0 지금 항목 기준. `timeScale`은 모든 트랙 dt에 곱한다. `Completed`는 트랙 0 `TrackCompletion`에서, `TrackCompleted`는 모든 트랙에서.
- 없는 애니메이션 이름: `Debug.LogWarning` 한 번(이름별), 트랙 그대로, null 반환.

- [ ] **Step 1: 실패하는 EditMode 테스트:**
  - `m20_existing_play_unchanged` — 기존 m11 시나리오(Play·Stop·반복 아님 끝·이벤트)가 트랙 경로에서도 같은 값(기존 m11 테스트가 그대로 통과하는지로 확인 + Time 값 비교 하나).
  - `m20_walk_with_attack_overlay` — 트랙 0 걷기 + 트랙 1 공격(팔만 키) → 다리 본은 걷기만 돌렸을 때와 같고 팔 본은 공격 값; 공격이 끝나고 `AddEmpty(1, 0, 0.2)` 예약이면 0.2초 동안 팔이 걷기 값으로 돌아감.
  - `m20_animation_name_change_uses_default_mix` — `defaultMix=0.2`에서 플레이 중 `animationName` 변경 → 다음 프레임 포즈가 두 애니메이션 사이.
  - `m20_track_completed_and_event_track` — 트랙 1 반복 아님: `TrackCompleted(1, 이름)` 한 번, 그 애니메이션 이벤트의 `Track == 1`, `Animation == 이름`.
  - `m20_unknown_animation_keeps_track`(Review Focus 1).
  테스트용 모델은 `tools/gen-samples.mjs`에 `unity-stage3-tracks.kgbone`(걷기·달리기·공격(팔만 키, 이벤트 "타격")·대기)을 더해 쓴다(정답 대상은 아님, 시험 파일만).
- [ ] **Step 2:** `npm run unity:editmode` → FAIL. **Step 3:** 구현. **Step 4:** `npm run unity:editmode`, `npm run unity:test` → PASS.
- [ ] **Step 5: 문서.** SPEC M19·M20, 패키지 README에 "트랙과 섞기" 절(걷기 + 공격 예제 코드: `SetAnimation(0,"걷기",true)`, 공격 때 `SetAnimation(1,"공격",false,0.1)` + `AddEmpty(1,0,0.2)`, 매 프레임 부를 때 `GetTrackAnimation`으로 확인), 지원 표, CHANGELOG, GAPS G-26(섞기·트랙 지원, 트랙 더하기·본 마스크 없음), HANDOFF.
- [ ] **Step 6: 커밋·push** `[ADD] 유니티 런타임: 컴포넌트 번호 트랙 API와 트랙 이벤트, 3단계 묶음 ② 문서`. push, CI 3개 확인.

## 묶음 ③ 첨부물 · 스킨 합치기 · 본 덮어쓰기

### Task 7: 계산부 스킨 목록과 덮어쓰기

**Files:** Create/채움 `Pose/SkinSet.cs`, Create `Pose/PoseOverrides.cs`; Modify `AnimationApplier.cs`, `ActiveBones.cs`, `ConstraintSolver.cs`(ActiveConstraints, SolveContext.Skin), `PathSolver.cs`, `SliderSolver.cs`, `DrawListBuilder.cs`, `SkeletonPoser.cs`, `PoseResult.cs`; Test `Stage3UnitTests.cs`

**Interfaces:**
- Produces: `public sealed class SkinSet { static SkinSet Single(string skin); static SkinSet Of(IReadOnlyList<string> skins); IReadOnlyList<string> Names; string Primary; AttachmentData Find(SkeletonModel m, string slot, string name) /* 뒤 스킨 → … → 앞 스킨 → 기본 스킨 */; bool Includes(string skin) /* deform·시퀀스 키 범위 */; }`. 지금 `string skin`을 받는 계산부 함수(위 Modify 목록)를 모두 `SkinSet`으로 바꾼다. 스킨 전용 본·제약 활성: 켠 스킨 중 하나라도 가지면 켜짐. `PoseRequest.Skins : string[]`(null이면 `Skin` 하나) → 내부 `SkinSet`(요청마다 새로 만들지 않게 캐시).
- Produces:
  ```csharp
  public sealed class BoneOverride { public float? Rotation, X, Y, ScaleX, ScaleY; public float Mix = 1; }
  public sealed class PoseOverrides
  {
      public void SetAttachment(int slot, string attachment); // null = 숨김
      public void ClearAttachment(int slot);
      public void SetBoneLocal(int bone, BoneOverride value);
      public void SetBoneWorldPosition(int bone, float x, float y, float mix);   // 뼈대 좌표
      public void SetBoneWorldRotation(int bone, float degrees, float mix);      // 뼈대 좌표의 본 X축 방향
      public void ClearBone(int bone);
      public void ClearAllBones();
      public bool IsEmpty { get; }
  }
  ```
  `PoseRequest.Overrides : PoseOverrides`(기본 null).
- 순서(설계 5.1): 트랙/애니메이션 → 첨부물 덮어쓰기 → 로컬 본 덮어쓰기(정한 채널만, `Mix`로 섞기, 회전 짧은 쪽, 절대 로컬 값) → 활성 판정 → 월드 → 월드 덮어쓰기(본마다: 목표 월드 위치/회전을 2단계 `BoneTransform.WorldToLocal`·`SetWorld`로 로컬에 넣고 `UpdateFrom`으로 그 본과 자식 다시 계산, `mix`는 지금 월드 값과 목표 사이 선형/짧은 쪽) → 물리 reset → 제약 → 그리기.

- [ ] **Step 1: 실패하는 테스트:**
  - `m21_skinset_single_equals_string_path` — 정답 `spine43-skins`의 스킨별 결과가 `Skins = new[]{s}`와 같음.
  - `m21_later_skin_wins` — 두 스킨이 같은 (슬롯, 이름) 첨부물을 가지면 뒤 스킨 것.
  - `m21_skin_only_bone_active_if_any_skin` — 스킨 B 전용 본이 `Skins={A,B}`에서 켜짐.
  - `m21_attachment_override_beats_key_and_clears` — 첨부물 키가 있는 애니메이션 위에서도 덮어쓴 값, `ClearAttachment` 뒤 키 값, `null`이면 그 슬롯 안 그림.
  - `m22_local_override_mix_half` — 회전 키 90° 위에 `Rotation=0, Mix=0.5` → 45°.
  - `m22_world_position_drives_ik` — IK 대상 본을 `SetBoneWorldPosition`으로 옮기면 IK 사슬 끝이 그 점 쪽으로(정답 `spine43-ik` 모델 사용, 끝 본 월드 위치가 목표에 1e-2 이내 또는 IK 길이 한계).
  - `m22_world_rotation_with_parent_rotated` — 부모가 30° 돈 본에 월드 90° → 본 월드 회전 90°(1e-3), 로컬 60°.
  - `m22_constraint_wins_on_constrained_bone` — 트랜스폼 제약이 걸린 본을 덮어써도 제약 결과(설계 판단 확인).
- [ ] **Step 2:** FAIL. **Step 3:** 구현. **Step 4:** `npm run unity:test` PASS(정답 912개 포함).
- [ ] **Step 5: 커밋** `[ADD] 유니티 런타임: 스킨 합치기와 첨부물·본 덮어쓰기 계산부`.

### Task 8: 컴포넌트 API, 묶음 ③ 문서와 push

**Files:** Modify `Runtime/KGBoneAnimation.cs`, docs; Test `AnimationComponentTests.cs`

**Interfaces (공개, 이름 확정 — 설계 5.4):**
```csharp
public void SetAttachment(string slot, string attachment);
public void ClearAttachment(string slot);
public void SetSkins(params string[] skins);      // skin 필드 = 첫 값, 물리 초기화
public void SetBoneLocal(string bone, BoneOverride value);
public void SetBoneWorldPosition(string bone, Vector3 position, float mix = 1);
public void SetBoneWorldRotation(string bone, float degrees, float mix = 1);
public void ClearBone(string bone);
public void ClearAllBones();
```
- 유니티 월드 → 뼈대 좌표: `TryGetBone`의 반대(GameObject `worldToLocalMatrix`, `pixelsPerUnit`, `flipX`/`flipY`). 회전도 같은 변환(뒤집기면 각도 부호 반전 규칙을 `TryGetBone`과 맞춤).
- 이름 → 인덱스는 모델에서 찾고, 없으면 경고 한 번(이름별).
- 인스펙터 `skin` 필드를 직접 바꾸면 `SetSkins(skin)`과 같다(지금 동작 유지).

- [ ] **Step 1: 실패하는 EditMode 테스트:**
  - `m21_set_attachment_swaps_weapon` — 슬롯 "무기"에 다른 첨부물 → 다음 프레임 그리기 목록 이미지가 바뀜.
  - `m21_set_skins_combines` — `SetSkins("몸","칼")` → 두 스킨 첨부물이 모두 그려짐, `skin == "몸"`.
  - `m21_unknown_names_warn_once`(Review Focus 1: 같은 없는 이름 두 번 → 경고 한 번).
  - `m22_world_position_with_flip_and_scale` — GameObject 크기 2, `flipX` 켠 상태에서 `SetBoneWorldPosition(p)` 뒤 `TryGetBone`이 p를 돌려줌(1e-3).
  - `m22_world_rotation_follows_mouse_angle` — `SetBoneWorldRotation(b, 45)` 뒤 `TryGetBone` 회전 45°(뒤집기 끈 경우와 켠 경우).
  테스트 모델은 `unity-stage3-tracks.kgbone`에 스킨 "몸"·"칼"과 슬롯 "무기"를 더해 함께 쓴다.
- [ ] **Step 2:** FAIL. **Step 3:** 구현. **Step 4:** `npm run unity:editmode`, `npm run unity:test` → PASS.
- [ ] **Step 5: 문서.** SPEC M21·M22, 패키지 README "무기 바꾸기"(`SetAttachment`, `SetSkins`)와 "마우스 따라 고개 돌리기"(`SetBoneWorldRotation`, IK 대상 `SetBoneWorldPosition`) 예제, README 지원 표, CHANGELOG, GAPS G-26(3단계 남은 것: Animator·Timeline, 루트 모션, 트랙 더하기·마스크), HANDOFF(3단계 끝, 다음 후보), 사용 설명서 7장 유니티 부분(있으면).
- [ ] **Step 6: 커밋·push** `[ADD] 유니티 런타임: 첨부물·스킨 합치기·본 덮어쓰기 API와 3단계 문서`. push, CI 3개 확인.
