# KGBone 유니티 런타임 2단계(제약) 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 유니티 런타임 계산부가 제약 5종(IK·트랜스폼·슬라이더·패스·물리)과 제약 타임라인을 KGBone 핵심과 같게 계산하고, 컴포넌트에 물리용 월드 움직임 반영 옵션을 더한다.

**Architecture:** 핵심 `crates/kgbone-core/src/pose/{ik,transform,slider,path,physics}.rs`와 `anim/constraint_keys.rs`를 C# 9로 줄 단위 대응시켜 옮긴다. 정답은 `tools/unity-golden.mjs`가 kgbone.wasm으로 만들고 `GoldenTests`가 비교한다(물리는 한 엔진의 연속 요청). 세 묶음(①IK·트랜스폼·슬라이더 ②패스 ③물리+옵션)으로 나눠 묶음마다 push하고 CI를 확인한다.

**Tech Stack:** C# 9(유니티 호환), .NET 10 + NUnit(`npm run unity:test`), Unity 6000.3 + URP(EditMode), Node 24(정답 생성기).

**Spec:** `docs/superpowers/specs/2026-11-17-unity-runtime-stage2-design.md` (앞 단계: `2026-11-16-unity-runtime-stage1-design.md`, `docs/superpowers/plans/2026-11-16-unity-runtime-stage1.md`)

## Global Constraints

- 클린룸: 근거는 `docs/FORMAT.md`와 `crates/kgbone-core` 코드뿐. `EsotericSoftware/spine-runtimes`의 어떤 파일도 열지 않는다. 이름에 "Spine"을 쓰지 않는다.
- 계산부 `Runtime/Core`: UnityEngine 없음, 외부 의존성 0개, **C# 9 문법만**. 숫자는 핵심과 같은 `float`, 삼각함수 `MathF`. 핵심이 `f64`를 쓰는 곳(물리 `remaining`, fps 나눗셈)은 C#도 `double`.
- 핵심의 계산을 바꾸지 않는다. 핵심 버그로 보이면 C#은 핵심과 같게 옮겨 정답을 통과시키고 GAPS.md에 적어 보고한다.
- 정답 비교 오차 1e-3(물리 연속 정답 포함). 넘으면 먼저 계산 순서·float/double을 핵심과 맞추고, 그래도 넘으면 비교 프레임 수를 줄이고 GAPS.md에 적는다.
- 월드 움직임 반영 기본값 0(이동·회전 따로, 0~1). 0이면 핵심과 같은 경로.
- 물리 진행: 플레이 모드에서 컴포넌트가 켜져 있으면 항상, dt = `min(0.1, Time.deltaTime)`, 컴포넌트 `timeScale` 무관. 에디터 모드에서는 진행하지 않음. `Play()`·`Stop()`은 물리를 초기화하지 않음.
- 테스트 이름에 SPEC 번호: `m13`(제약 모델·키·순서·IK·트랜스폼·슬라이더), `m14`(패스), `m15`(물리), `m16`(월드 움직임 반영·컴포넌트 물리). Task 8에서 SPEC.md M장에 적는다.
- 커밋: `[ADD]/[CHG]/[DEL]/[FIX]` + 한국어, 공동 작업자 줄 금지. 묶음 끝(Task 4, 5, 8)마다 `git push origin HEAD:main`(강제 금지) 후 GitHub Actions 3개 워크플로 성공 확인. `.claude/`, `local/`, 골렘 파일은 커밋하지 않는다.
- 핵심 계산을 바꾸지 않으므로 Rust·화면·MCP 테스트는 영향이 없지만, 묶음 끝마다 `npm run unity:test`와 유니티 EditMode(KGBoneTest 또는 KGBoneSandbox, unityMCP `run_tests`)를 돌린다.

## Review Focus

1. **스킨 전용 제약(`skinRequired`)이 그 스킨이 아닐 때**: 계산하지 않아야 하고, 그 제약의 키도 포즈를 바꾸지 않아야 한다. → Task 2 테스트 `m13_skin_required_constraint_skipped_without_skin`.
2. **제약 대상 본이 비활성(스킨 전용 본)**: 핵심 `active_constraints`와 같이 건너뛴다. 예외 없음. → Task 2 같은 테스트에 경우 추가.
3. **슬라이더가 자기 자신의 키를 가진 애니메이션을 적용**: 무한 순환 없이 자기 키를 건너뛴다. → Task 4 `m13_slider_skips_own_keys`.
4. **물리 첫 프레임 dt가 큼(로딩 멈칫), 또는 dt 0**: 0.1초 상한, dt 0이면 상태 변화 없음, 튀지 않음. → Task 8 `m16_large_first_frame_clamped`.
5. **GameObject 순간이동 뒤 `ResetPhysics()`**: 다음 프레임 오프셋 0, 튐 없음. → Task 7 `m16_reset_clears_pending_world_motion`.

---

## 파일 구조

```
unity/com.kgbone.runtime/
  Runtime/Core/Model/
    ConstraintData.cs        (새) 제약 5종 데이터 클래스
    KGBoneLoader.cs          (고침) constraints·옛 배열 합치기·path 첨부물·제약 타임라인 읽기
    AttachmentData.cs        (고침) path 첨부물 필드
    AnimationData.cs         (고침) ConstraintTimeline 목록
    SkeletonModel.cs         (고침) Constraints, ConstraintIndex, 미지원 판정
  Runtime/Core/Anim/
    AnimationApplier.cs      (고침) alpha·additive·skipConstraint, 제약 키
    ConstraintKeys.cs        (새) anim/constraint_keys.rs 대응
  Runtime/Core/Pose/
    ConstraintSolver.cs      (새) IConstraintSolver, SolveContext, ActiveConstraints, 월드 다시 계산 도우미(set_world)
    IkSolver.cs, TransformSolver.cs, SliderSolver.cs, PathSolver.cs, PhysicsSolver.cs (새)
    PoseState.cs             (고침) ConstraintValues
    PoseResult.cs            (고침) PoseRequest.Physics, PoseRequest.WorldMotion
    SkeletonPoser.cs         (고침) 제약 루프, 물리 상태, ResetPhysics
  Runtime/KGBoneAnimation.cs (고침) 물리 진행·반영 옵션·ResetPhysics
  Editor/KGBoneImporter.cs   (고침) 미지원 제약 종류만 경고
  Tests/Editor/
    ConstraintUnitTests.cs   (새, dotnet test에도 포함: csproj의 Compile 목록에 추가)
    GoldenTests.cs, GoldenFile.cs (고침) 연속(stateful) 정답 파일
    AnimationComponentTests.cs, ImporterTests.cs (고침)
  Tests/Golden/              (다시 생성)
tools/unity-golden.mjs       (고침) 대상 추가, 물리 연속 요청
tools/gen-samples.mjs        (고침) unity-stage2-*.kgbone 시험 파일
samples/kgbone/unity-stage2-*.kgbone (생성물)
unity/tests/KGBone.Core.Tests/KGBone.Core.Tests.csproj (고침) ConstraintUnitTests*.cs 포함
```

---

## 묶음 ① IK · 트랜스폼 · 슬라이더

### Task 1: 제약 데이터와 불러오기

**Files:** Create `Model/ConstraintData.cs`; Modify `KGBoneLoader.cs`, `AttachmentData.cs`, `AnimationData.cs`, `SkeletonModel.cs`, csproj; Test `Tests/Editor/ConstraintUnitTests.cs`

**Interfaces:**
- Produces: `abstract class ConstraintData { string Name; bool SkinRequired; abstract ConstraintType Type; }` + `IkConstraintData`, `TransformConstraintData`(+ `TransformMapping { Property From; float Offset; List<TransformTo> To }`, `TransformTo { Property Property; float Offset, Max, Scale }`), `PathConstraintData`, `PhysicsConstraintData`, `SliderConstraintData`. 필드 이름·기본값은 FORMAT.md 3.3과 핵심 `model`(`crates/kgbone-core/src/model/`)의 같은 구조체를 따른다(본·슬롯은 이름 대신 인덱스 `int`로 풀어 둔다, 못 찾으면 -1).
- Produces: `SkeletonModel.Constraints : List<ConstraintData>`, `int ConstraintIndex(string name)`.
- Produces: `PathAttachmentData : AttachmentData { bool Closed, ConstantSpeed; int VertexCount; float[] Vertices; bool Weighted; float[] Lengths; }`(그리지 않음).
- Produces: `sealed class ConstraintTimeline { int Constraint; string Name; Key[] Keys; }`, `AnimationData.Constraints : List<ConstraintTimeline>` — 채널 이름은 FORMAT.md 3.7 표 그대로(`reset` 포함). 길이(`Duration`)에 제약 키 시간도 포함(1단계 그대로인지 확인).
- `ConstraintCount`는 지우고, 경고는 Task 4·5·8에서 `SkeletonModel.UnsupportedConstraintTypes()`(묶음마다 줄어드는 목록)로 한다. 이번 Task에서는 5종 모두 미지원으로 돌려준다.

- [ ] **Step 1: 실패하는 테스트** (`ConstraintUnitTests`, 작은 .kgbone 텍스트를 테스트 안에 직접 씀):
  - `m13_loads_ik_with_defaults`: `{"type":"ik","name":"k","bones":["a"],"target":"t"}` → Mix 1, Softness 0, BendPositive true, Compress·Stretch·Uniform·Volume false, Bones = [a의 인덱스], Target = t의 인덱스.
  - `m13_loads_transform_default_mappings`: properties 없음 → 6개 속성 각자 같은 속성 매핑(offset 0, scale 1, max 0), 모든 mix 1.
  - `m13_loads_physics_and_slider_fields`: physics의 fps·limit·inertia… , slider의 animation·bone·property·from/to·loop·additive가 파일 값 그대로.
  - `m13_legacy_arrays_merged_by_order`: formatVersion 0, `ik`(order 2)·`transform`(order 0)·`physics`(order 1) → Constraints 순서 [transform, physics, ik] (핵심 `format/migrate.rs`와 같게).
  - `m13_constraint_timelines_loaded`: 애니메이션 `constraints: {"k": {"mix": [...], "reset": [...]}}` → ConstraintTimeline 2개, Constraint = k의 인덱스.
  - `m14_loads_path_attachment`: path 첨부물 필드가 읽힌다.
- [ ] **Step 2:** `npm run unity:test` → 새 테스트 실패 확인(컴파일 오류 아님: 먼저 빈 클래스로 컴파일되게 한 뒤 값 비교로 실패).
- [ ] **Step 3:** 구현. 로더는 핵심의 읽기(`format/kgbone.rs`, `model`)와 같은 기본값을 쓴다.
- [ ] **Step 4:** `npm run unity:test` 전부 통과(기존 정답 포함).
- [ ] **Step 5:** 커밋 `[ADD] 유니티 런타임: 제약·패스 첨부물·제약 타임라인 불러오기`.

### Task 2: 제약 값·제약 키·섞기 적용·적용 순서 틀

**Files:** Create `Anim/ConstraintKeys.cs`, `Pose/ConstraintSolver.cs`; Modify `AnimationApplier.cs`, `PoseState.cs`, `SkeletonPoser.cs`, `ActiveBones.cs`(활성 제약 판정을 넣을 곳으로 적당하면); Test `ConstraintUnitTests.cs`

**Interfaces:**
- Consumes: Task 1의 모델.
- Produces: `PoseState.ConstraintValues : float[][]` — 핵심 `constraint_values`와 같은 모양(ik 5개, transform 6개, path 5개, physics 7개, slider 2개), 셋업 값은 `constraint_setup_values`와 같게. `ResetToSetup`이 되돌린다.
- Produces: `AnimationApplier.Apply(SkeletonModel m, AnimationData a, float time, string skin, PoseState pose, float alpha = 1, bool additive = false, int skipConstraint = -1)` — 핵심 `anim/apply.rs`의 `apply_scoped`·`ApplyOptions` 의미 그대로(alpha·additive가 본·슬롯·deform·제약 키 각각에 주는 효과를 핵심 코드에서 옮김). 기본 인자면 1단계와 같은 결과.
- Produces: `static class ConstraintKeys { static void Apply(SkeletonModel m, AnimationData a, float time, PoseState pose, float alpha, bool additive, int skip) }` — `anim/constraint_keys.rs` 대응.
- Produces: `interface IConstraintSolver { void Apply(SkeletonModel m, int index, PoseState pose, SolveContext ctx); }`, `sealed class SolveContext { string Skin; PhysicsStep Physics; int[] Parents; }`, `static bool[] ActiveConstraints(SkeletonModel m, bool[] activeBones, string skin)`(`pose/active.rs`), 월드 갱신 도우미 `BoneTransform.SetWorld(int[] parents, PoseState pose, int bone, in WorldBone w)`·`BoneTransform.UpdateFrom(SkeletonModel m, int[] parents, PoseState pose, int start)`(`bone_transform.rs`의 `set_world`·`update_world`).
- `SkeletonPoser.Compute` 순서: 셋업 → 애니메이션 → 활성 → 월드 → (Task 6에서 물리 reset) → 활성 제약 배열 순서대로 solver → 그리기 → 이벤트. 이 Task에서는 solver가 없는 종류는 건너뛴다.

- [ ] **Step 1: 실패하는 테스트**
  - `m13_constraint_key_sets_ik_mix`: ik mix 키 0초 1 → 1초 0(선형), 0.5초 → `ConstraintValues[k][0] ≈ 0.5`.
  - `m13_alpha_half_blends_bone_rotation`: 셋업 회전 0, 키 회전 40, alpha 0.5 → 로컬 회전 20(핵심 `apply.rs` 테스트가 있으면 그 값).
  - `m13_additive_adds_on_current_pose`: 핵심 apply.rs의 additive 규칙대로 기대값을 손으로 계산.
  - `m13_skin_required_constraint_skipped_without_skin` (Review Focus 1·2): skinRequired 제약은 그 스킨이 켜졌을 때만 `ActiveConstraints`가 true. 대상 본이 비활성이면 false.
  - 기존 정답 전부 그대로 통과(기본 인자 경로가 1단계와 같음).
- [ ] **Step 2:** 실패 확인 → 구현 → `npm run unity:test` 통과.
- [ ] **Step 3:** 커밋 `[ADD] 유니티 런타임: 제약 값과 제약 키, 비율·더하기 적용, 제약 적용 순서`.

### Task 3: IK와 트랜스폼

**Files:** Create `Pose/IkSolver.cs`, `Pose/TransformSolver.cs`; Modify `tools/unity-golden.mjs`(TARGETS에 `spine43-ik`·`spine43-transform`), `Tests/Golden/*`(생성물); Test `ConstraintUnitTests.cs`

**Interfaces:**
- Consumes: Task 2의 `IConstraintSolver`, `SetWorld`/`UpdateFrom`.
- Produces: `IkSolver`, `TransformSolver`(`pose/ik.rs`, `pose/transform.rs` 대응). `TransformSolver.BoneProperty(PoseState pose, int bone, Property p, bool local) : float`는 슬라이더가 쓴다(`transform.rs`의 `bone_property`).

- [ ] **Step 1: 실패하는 테스트**: 핵심 `ik.rs`·`transform.rs`의 `#[cfg(test)]` 테스트를 같은 입력·기대값으로 옮긴다(이름은 `m13_ik_<핵심 테스트 이름>`, `m13_transform_<…>`). 정답 생성기에 두 파일을 넣고 `npm run unity:golden` → `npm run unity:test`에서 새 정답 케이스가 실패하는 것 확인.
- [ ] **Step 2:** 구현 → `npm run unity:test` 전부 통과.
- [ ] **Step 3:** 커밋 `[ADD] 유니티 런타임: IK·트랜스폼 제약 계산과 정답 비교`.

### Task 4: 슬라이더, 섞은 시험 파일, 묶음 ① 마무리

**Files:** Create `Pose/SliderSolver.cs`; Modify `tools/gen-samples.mjs`(새 `samples/kgbone/unity-stage2-slider.kgbone`, `unity-stage2-mixed.kgbone`), `tools/unity-golden.mjs`, `SkeletonModel.UnsupportedConstraintTypes`(ik·transform·slider 빠짐), `Editor/KGBoneImporter.cs`, `ImporterTests.cs`, 문서(README 지원 표, 패키지 README·CHANGELOG, GAPS G-26)

**Interfaces:**
- Consumes: `TransformSolver.BoneProperty`, `AnimationApplier.Apply(…, alpha, additive, skipConstraint)`.
- 시험 파일 내용(덮을 경우):
  - slider: ① bone 없이 time 키로 움직이는 슬라이더 ② bone 연결(local true / false 각각) ③ loop true(길이 넘는 값) / loop false(범위 밖은 자름) ④ additive true ⑤ 적용하는 애니메이션 안에 자기 자신 키(Review Focus 3).
  - mixed: 제약 순서 ik → transform → slider로 앞 결과를 뒤가 읽게, 하나는 skinRequired + 스킨 하나.
  - 두 파일 모두 클리핑·시퀀스 없음.

- [ ] **Step 1: 실패하는 테스트**: `m13_slider_skips_own_keys`(위 ⑤에서 예외·무한 반복 없이 끝나고 정답과 같음), 핵심 `slider.rs` 테스트 옮기기, `m10_importer_warns_only_unsupported_constraints`(ik 파일은 제약 경고 없음, path 파일은 "패스" 경고 있음). 정답 생성 → 실패 확인.
- [ ] **Step 2:** 구현 → `npm run samples`, `npm run unity:golden`, `npm run unity:test` 통과 → 유니티 EditMode 통과(unityMCP `run_tests`, 테스트 프로젝트에 패키지가 로컬 경로로 연결돼 있어야 함).
- [ ] **Step 3:** 문서 고침(지원 표: IK·트랜스폼·슬라이더 지원, 패스·물리 예정). 커밋 `[ADD] 유니티 런타임: 슬라이더 제약과 2단계 시험 파일, 가져오기 경고 갱신` → push → CI 3개 성공 확인.

## 묶음 ② 패스

### Task 5: 패스 제약

**Files:** Create `Pose/PathSolver.cs`; Modify `tools/gen-samples.mjs`(`unity-stage2-path.kgbone`: closed true, constantSpeed false, 가중치 패스, spacing 모드·rotate 모드 기본값 아닌 것 — `pose/path.rs`가 분기하는 옵션을 하나 이상씩), `tools/unity-golden.mjs`(`spine43-path` + 새 파일), `UnsupportedConstraintTypes`(path 빠짐), 문서; Test `ConstraintUnitTests.cs`

**Interfaces:**
- Consumes: Task 1 `PathAttachmentData`, Task 2 solver 틀, 1단계 `MeshVertices`(가중치 정점 월드 좌표 계산을 재사용할 수 있으면).
- Produces: `PathSolver`(`pose/path.rs` 대응). 패스 첨부물은 슬롯의 현재 첨부물을 고른 스킨 → 기본 스킨 순서로 찾고, deform 키도 반영(핵심과 같게).

- [ ] **Step 1: 실패하는 테스트**: 핵심 `path.rs` 테스트 옮기기(`m14_path_<…>`). 정답 생성 → 실패 확인.
- [ ] **Step 2:** 구현 → `npm run unity:test` 통과 → EditMode 통과.
- [ ] **Step 3:** 커밋 `[ADD] 유니티 런타임: 패스 제약 계산과 정답 비교` → push → CI 확인.

## 묶음 ③ 물리와 월드 움직임 반영

### Task 6: 물리 제약과 연속 정답

**Files:** Create `Pose/PhysicsSolver.cs`; Modify `PoseResult.cs`(PoseRequest), `SkeletonPoser.cs`, `tools/unity-golden.mjs`, `GoldenFile.cs`, `GoldenTests.cs`, `UnsupportedConstraintTypes`(physics 빠짐 → 빈 목록)

**Interfaces:**
- Produces: `struct PhysicsStep { static PhysicsStep None; static PhysicsStep Reset; static PhysicsStep Advance(float dt); }`, `PoseRequest.Physics : PhysicsStep`(기본 None).
- Produces: `SkeletonPoser.ResetPhysics()` — 상태를 모두 비움(`PoseEngine::reset_physics`).
- `Compute`: 월드 계산 뒤 `Physics == Reset`이면 전부 초기화, `LastTime`이 있으면 `reset` 키가 `[LastTime, Time)`에 든 제약만 초기화(`events::physics_resets`, 구간 규칙은 1단계 `EventFinder`의 hits와 같음).
- 물리 상태: 제약 이름별 `{ initialized, origin Point, tip Point, double remaining }`(`physics.rs`의 `ConstraintState`).
- 정답 파일 형식 확장: 최상위 `"stateful": true`이면 케이스를 **한 SkeletonPoser로 순서대로** 계산하고 요청의 `"physics"`(`"none"`/`"reset"`/숫자)를 그대로 넘긴다. 생성기는 파일 시작 때 `resetPhysics`를 부르고 같은 순서로 `pose`를 부른다. GoldenTests는 stateful 파일을 파일 하나당 테스트 하나(`m15_golden_sequence_<파일>`)로, 첫 차이의 프레임 번호와 경로를 메시지에 넣는다.
- 물리 연속 요청(`spine43-physics`, 애니메이션마다): 1/60초 × 120프레임(반복 재생, lastTime 포함) → 중간에 `"reset"` 한 번 → 불규칙 dt(0.004, 0.033, 0.1, 0, 0.017 반복) 60프레임. reset 키가 있는 애니메이션이면 그 키를 지나는 구간이 들어가게 한다(시험 파일에 없으면 gen-samples로 `unity-stage2-physics.kgbone`을 만들어 reset 키·바람·중력·volume·길이 0 본을 덮는다).

- [ ] **Step 1: 실패하는 테스트**: 핵심 `physics.rs` 테스트 옮기기(`m15_physics_<…>`: 정지면 변화 없음, 중력이면 처짐, reset 뒤 처음 상태 등), `m15_reset_key_resets_named_constraint_only`. 정답 생성 → 실패 확인.
- [ ] **Step 2:** 구현 → `npm run unity:test` 통과. 오차를 넘으면 Global Constraints의 순서대로 처리하고 기록.
- [ ] **Step 3:** 커밋 `[ADD] 유니티 런타임: 물리 제약 계산과 연속 정답 비교`.

### Task 7: 월드 움직임 입력(계산부)

**Files:** Modify `PoseResult.cs`, `PhysicsSolver.cs`; Test `ConstraintUnitTests.cs`

**Interfaces:**
- Produces: `struct WorldMotion { float X, Y; float Rotation; }`(뼈대 좌표계, 이번 프레임에 GameObject가 움직인 양 × 반영 값. 회전은 도, 뼈대 원점 기준) — `PoseRequest.WorldMotion`(기본 0).
- 물리 계산: 질점마다 이번 프레임 이동량 = (본 이동량) + (WorldMotion이 그 질점의 지난 위치를 옮긴 양: 평행 이동 + 원점 기준 회전으로 생긴 차이). 그 뒤는 본 이동량과 똑같이 inertia가 곱해지고, 진행 단계가 0이면 다음 단계로 미뤄진다(미뤄진 월드 움직임을 상태에 쌓음). WorldMotion이 0이면 덧셈을 하지 않아 정답과 같은 경로.
- `ResetPhysics()`는 쌓인 월드 움직임도 비운다.

- [ ] **Step 1: 실패하는 테스트**
  - `m16_zero_world_motion_matches_golden`: 물리 정답 파일을 WorldMotion 0으로 돌리면 Task 6과 같은 결과(GoldenTests 재사용 또는 같은 값 확인).
  - `m16_move_right_makes_tip_lag_left`: 중력·바람 0, 정지 본, WorldMotion X +10 한 프레임 후 몇 프레임 → 끝 질점 offset.x < 0, 본 회전 결과가 처짐 방향.
  - `m16_half_factor_halves_first_offset`: 같은 조건에서 X 5와 X 10의 첫 단계 offset 비가 0.5(±1e-4, 선형인 첫 단계에서 비교).
  - `m16_rotation_motion_moves_tip`: Rotation 10도 → 원점에서 떨어진 끝 질점에 offset 생김, 원점 질점(원점 = 뼈대 원점일 때)은 0.
  - `m16_reset_clears_pending_world_motion` (Review Focus 5): dt 0으로 WorldMotion을 쌓은 뒤 `ResetPhysics()` → 다음 Advance에서 offset 0.
- [ ] **Step 2:** 실패 확인 → 구현 → `npm run unity:test` 통과.
- [ ] **Step 3:** 커밋 `[ADD] 유니티 런타임: 물리에 월드 움직임 반영 입력 추가`.

### Task 8: 컴포넌트 물리, 문서, 묶음 ③ 마무리

**Files:** Modify `Runtime/KGBoneAnimation.cs`, `Editor/KGBoneImporter.cs`(제약 경고 없음), `Tests/Editor/AnimationComponentTests.cs`, `ImporterTests.cs`, 문서(README, 패키지 README·CHANGELOG(0.2.0 항목, 태그는 만들지 않음), docs/SPEC.md M장에 M13~M16, GAPS G-26 고침, HANDOFF, `manual/`에 유니티 관련 내용이 있으면 해당 부분)

**Interfaces:**
- Consumes: `PoseRequest.Physics`, `PoseRequest.WorldMotion`, `SkeletonPoser.ResetPhysics()`.
- Produces(컴포넌트 공개 API): `[Range(0,1)] public float physicsMoveFactor = 0; [Range(0,1)] public float physicsRotateFactor = 0; public void ResetPhysics();`
- 매 프레임(플레이 모드, 컴포넌트 켜짐, `Advance` 경로): dt = `min(0.1f, Time.deltaTime)`(컴포넌트 `timeScale` 무관, `IsPlaying`과 무관) → 지난 프레임 대비 `transform` 위치·Z 회전 변화를 뼈대 좌표로 바꿈(pixelsPerUnit, lossyScale, flipX/flipY 반영; 회전은 뒤집기 한 번이면 부호 반대) → × 반영 값 → `WorldMotion`. 위치 기록은 매 프레임 갱신.
- 정지 상태(`IsPlaying` false)에서도 물리를 진행할 때는 마지막 애니메이션 시점의 포즈로 다시 계산한다(멈춘 시점 유지, 이벤트 없음).
- `OnEnable`, 데이터 다시 가져오기(`OnDataReimported`), 데이터·스킨 바꿈 때 `ResetPhysics()`. `ResetPhysics()`는 지난 위치 기록도 지금 위치로 맞춘다.
- 에디터 모드에서는 `Physics = None`.

- [ ] **Step 1: 실패하는 EditMode 테스트**(플레이 모드 진입, 데이터는 메모리에서 만듦 — 1단계 `m11_spawned_from_code_plays_on_start` 방식):
  - `m16_physics_advances_in_play_mode`: 중력 있는 물리 본이 몇 프레임 뒤 셋업 위치에서 벗어남.
  - `m16_physics_continues_after_stop`: `Stop()` 뒤에도 프레임마다 물리 본 위치가 바뀌다 가라앉음.
  - `m16_move_factor_zero_ignores_transform_move`: 반영 0이면 GameObject를 옮겨도 뼈대 좌표의 본 결과가 안 옮긴 것과 같음.
  - `m16_move_factor_one_reacts_to_transform_move`: 반영 1이면 다름.
  - `m16_large_first_frame_clamped` (Review Focus 4): `Advance` 경로에 dt 1.0을 넣으면 0.1과 같은 결과(물리 진행분 기준; 이 비교를 위해 컴포넌트 내부 물리 dt 계산을 `internal static float PhysicsDt(float deltaTime)`로 빼서 단위 확인해도 됨).
  - `m10_importer_no_constraint_warning`: 제약 5종이 든 파일 가져오기에 제약 경고 없음(클리핑·시퀀스 경고는 그대로).
- [ ] **Step 2:** 실패 확인 → 구현 → EditMode 전부 통과, `npm run unity:test` 통과.
- [ ] **Step 3:** 문서 고침. 커밋 `[ADD] 유니티 런타임: 컴포넌트 물리 진행과 월드 움직임 반영 옵션, 2단계 문서` → push → CI 3개 성공 확인.
- [ ] **Step 4:** KGBoneTest 프로젝트에서 눈으로 확인: 물리 시험 파일을 놓고 플레이, 반영 0/1에서 GameObject를 움직여 흔들림 차이를 캡처(저장소에 넣지 않음).
