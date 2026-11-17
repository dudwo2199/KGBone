# KGBone Runtime (com.kgbone.runtime)

[KGBone](https://github.com/dudwo2199/KGBone)(2D 스켈레탈 애니메이션 에디터)의 `.kgbone` 파일을 유니티에서 재생한다.
계산 결과는 KGBone 에디터와 같다(KGBone 핵심이 만든 정답과 자동 비교).

- 유니티 6000.0 이상, URP
- 1단계: 재생 + 본 위치 읽기

## 설치

Package Manager → **+** → **Add package from git URL**

```
https://github.com/dudwo2199/KGBone.git?path=/unity/com.kgbone.runtime#main
```

아직 버전 태그와 라이선스가 정해지지 않았다(배포 전).

## 가져오기

`.kgbone` 파일과 그 이미지 폴더(파일의 `imagesPath`, 기본 `./images`)를 함께 `Assets` 아래에 넣는다.
가져오기 도구가 자동으로 만든다:

- **KGBoneSkeletonData**(메인 에셋): 뼈대 데이터(편집 상태 제외), 가져오기 경고
- 파츠를 한 장으로 묶은 텍스처(알파 미리 곱함, 여백 2px, 최대 4096)
- 슬롯이 쓰는 블렌드 모드(normal, additive, multiply, screen)마다 머티리얼(셰이더 `KGBone/Unlit`)

인스펙터(.kgbone 파일 선택): **Pixels Per Unit**(기본 100), **Max Texture Size**, **Filter Mode**.
없는 이미지(시퀀스는 프레임마다 확인)는 경고로 알려 준다. 시퀀스 첨부물은 모든 프레임 이미지를 묶는다.
KGBone에서 고쳐 저장한 `.kgbone`으로 덮어쓰면 다시 가져오고, 씬의 캐릭터도 재생하지 않은 상태에서 바로 새 내용으로 그려진다.

## 재생: KGBoneAnimation

빈 GameObject에 **KGBoneAnimation**을 붙인다(MeshFilter·MeshRenderer가 함께 붙는다).

| 항목 | 뜻 |
|---|---|
| Skeleton Data | 가져온 데이터 |
| Skin | 켤 스킨(비우면 기본 스킨) |
| Animation Name | 시작할 애니메이션(비우면 셋업 포즈). 에디터에서는 이 애니메이션의 0초가 보인다 |
| Loop / Time Scale / Play On Awake | 반복 / 속도 / 시작하자마자 재생 |
| Flip X / Flip Y | 뒤집기 |
| Sorting Layer Name / Sorting Order | 다른 스프라이트와 그리기 순서 |
| Physics Move Factor / Physics Rotate Factor | 물리 제약이 GameObject 이동·회전에 반응하는 비율(0~1, 기본 0 = 에디터와 같음) |

```csharp
var anim = GetComponent<KGBoneAnimation>();
anim.Play("걷기", loop: true);       // 없는 이름이면 경고만 하고 그대로
anim.timeScale = 1.5f;
anim.EventFired += e => Debug.Log($"{e.Name} {e.Int} {e.String}");
anim.Completed += () => anim.Play("정지", true);   // 반복이 아닌 애니메이션이 끝났을 때
anim.Stop();                         // 셋업 포즈
```

코드로 만들 때는 `AddComponent` 뒤 같은 프레임에 데이터와 이름을 넣으면 Play On Awake로 자동 재생된다(자동 재생은 `Start`에서 한다). 그 전에 `Play`·`Stop`을 직접 부르면 자동 재생하지 않는다.

```csharp
var golem = new GameObject("골렘").AddComponent<KGBoneAnimation>();
golem.skeletonData = golemData;
golem.animationName = "idle";
```

이벤트의 `Time`은 키에 적힌 시간(애니메이션 안의 시간)이다. 이벤트 처리기 안에서 `Play`·`Stop`을 불러도 된다.
반복하지 않는 재생은 끝 시간의 키도 낸다. 반복 재생에서는 애니메이션 길이와 같은 시간의 키가 프레임 경계에 따라 나지 않을 수 있으므로, 끝 이벤트는 길이보다 조금 앞에 둔다. `timeScale`이 음수면 이벤트를 내지 않는다.

선형 색 공간(URP 기본)에서도 불투명한 부분은 KGBone과 같은 색이다. additive·screen 블렌드와 반투명 가장자리는 조금 다를 수 있다.

## 트랙과 섞기

애니메이션은 번호 트랙에 놓는다. `Play(이름, 반복)`은 트랙 0에 바로(섞지 않고) 놓는 것과 같다.
- 같은 트랙에서 바꾸면 `mix`초 동안 지금 포즈에서 새 포즈로 섞여 넘어간다(`mix`를 안 주면 인스펙터의 **Default Mix**, 기본 0 = 바로).
- 윗 트랙은 아랫 트랙 결과 위에 **자기 키가 있는 값만** 덮는다. 공격 애니메이션에 팔 키만 있으면 다리는 아랫 트랙(걷기)을 계속 따른다.
- `AddAnimation`은 지금 애니메이션이 끝나면(반복이면 이번 바퀴 끝) `delay`초 뒤 시작하도록 예약한다. `AddEmpty`·`ClearTrack`은 트랙을 서서히 비운다.
- 섞는 도중에 또 바꾸면 그 순간의 모습에서 새로 섞는다(튀지 않음).
- 이벤트(`EventFired`)에는 `Track`(트랙 번호)과 `Animation`(애니메이션 이름)이 들어 있다. 섞이며 빠지는 애니메이션의 이벤트는 내지 않는다.
- `TrackCompleted(트랙, 이름)`은 반복 아닌 애니메이션이 끝에 닿을 때 한 번. `Completed`는 트랙 0에 대해서만.
- 트랙·섞기는 플레이 중 상태라 씬에 저장되지 않는다. 에디터 모드 미리보기는 지금처럼 인스펙터의 Animation Name 0초다.

```csharp
// 걷기·달리기는 트랙 0, 공격은 트랙 1에 겹친다
anim.defaultMix = 0.15f;
anim.SetAnimation(0, "걷기", true);
void OnRun(bool running)
{
    string want = running ? "달리기" : "걷기";
    if (anim.GetTrackAnimation(0) != want) anim.SetAnimation(0, want, true); // 같은 이름을 다시 부르면 처음부터 시작하므로 확인하고 부른다
}
void OnAttack()
{
    anim.SetAnimation(1, "공격", false, 0.1f); // 팔만 공격 동작
    anim.AddEmpty(1, 0, 0.2f);                 // 끝나면 0.2초 동안 빠지며 걷기 팔로 돌아감
}
anim.EventFired += e => { if (e.Track == 1 && e.Name == "타격") Hit(); };
```

## 제약과 물리

제약 5종(IK, 트랜스폼, 패스, 물리, 슬라이더)과 제약 키는 KGBone 에디터와 같게 계산된다.

물리 제약(머리카락·옷자락 흔들림)은 플레이 모드에서 매 프레임 진행한다.
- 진행 시간은 실제 프레임 시간(`Time.deltaTime`, 한 프레임 최대 0.1초)이다. 유니티 전체 `Time.timeScale`을 낮추면 같이 느려지고, 컴포넌트의 `timeScale`(애니메이션 속도)과는 상관없다.
- 애니메이션이 멈춰도(`Stop()` 뒤, 반복 아닌 애니메이션이 끝난 뒤) 흔들림은 계속 진행해 가라앉는다. 에디터는 재생을 멈추면 물리도 멈추므로 이 부분만 다르다.
- 에디터 모드(플레이 아님)에서는 진행하지 않는다.
- `Play()`로 애니메이션을 바꾸거나 `Stop()`해도 흔들림을 초기화하지 않는다. 컴포넌트가 켜질 때, 데이터를 다시 가져올 때, 스킨을 바꿀 때는 초기화한다.

에디터에는 "캐릭터 자체가 움직이는" 상황이 없어서, GameObject를 옮기거나 돌려도 기본으로는 흔들림에 들어가지 않는다(에디터와 같은 결과).
걷기 이동처럼 GameObject를 움직일 때 흔들리게 하려면 **Physics Move Factor**(이동)와 **Physics Rotate Factor**(회전)를 0~1로 올린다.
이 움직임은 애니메이션이 본을 움직인 것과 똑같이 다뤄진다(물리 제약의 관성 inertia가 곱해진다). 0보다 큰 값의 흔들림은 에디터에서 미리 볼 수 없으므로 유니티에서 보면서 고른다.

```csharp
anim.physicsMoveFactor = 1f;
anim.transform.position = spawnPoint;   // 순간이동
anim.ResetPhysics();                    // 튀지 않게 흔들림과 지난 위치 기록을 처음으로
```

## 본 위치 읽기

```csharp
if (anim.TryGetBone("손", out Vector3 position, out float rotation, out Vector2 scale))
    effect.transform.position = position;   // 유니티 월드 좌표, 회전은 도
```

코드 없이 붙이려면 이펙트·무기 GameObject에 **KGBoneBoneFollower**를 붙이고 Target과 Bone Name을 고른다(Follow Rotation을 켜면 회전도 따라간다).

## 지원 범위 (3단계 진행 중)

| 지원 | 아직 없음 |
|---|---|
| 본(상속 5종), 이미지·메시·가중치 메시·링크드 메시, 메시 변형 키 | |
| 제약: IK(1본·2본, 부드러움·늘이기·줄이기·부피), 트랜스폼(속성 매핑), 패스(열림·닫힘, 가중치, 변형 키, 위치·간격·회전 방식), 물리(reset 키, 월드 움직임 반영 옵션), 슬라이더, 제약 값 키, 스킨 전용 제약 | |
| 슬롯 색·어두운 색, 첨부물 키, 그리기 순서 키, 블렌드 4종, 클리핑(보통·반전·볼록), 시퀀스(모드 7종) | |
| 스킨 1개 + 스킨 전용 본, 이벤트, 선형·계단·베지어 곡선 | 스킨 합치기, 첨부물 바꾸기, 본 덮어쓰기 — 3단계 진행 중 |
| 번호 트랙, 섞기·전환, 예약(`AddAnimation`·`AddEmpty`), 트랙 비율, 트랙별 이벤트·끝남 | 트랙 더하기(additive) 섞기, 본 마스크, Animator·Timeline 연동, 루트 모션 |

## 개발

- 계산부 `Runtime/Core`는 유니티 없이 컴파일되는 순수 C#(C# 9)이다. 저장소 루트에서 `npm run unity:test`(dotnet test)가 `Tests/Golden`의 정답과 비교한다.
- 정답은 `npm run unity:golden`이 KGBone 핵심(kgbone.wasm)으로 만든다.
- 유니티 EditMode 테스트: `npm run unity:editmode`(확인용 프로젝트 `unity/KGBoneSandbox`).
