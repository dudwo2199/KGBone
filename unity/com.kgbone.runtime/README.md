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
1단계에서 지원하지 않는 기능(제약, 클리핑, 시퀀스)이나 없는 이미지는 경고로 알려 준다.
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

## 본 위치 읽기

```csharp
if (anim.TryGetBone("손", out Vector3 position, out float rotation, out Vector2 scale))
    effect.transform.position = position;   // 유니티 월드 좌표, 회전은 도
```

코드 없이 붙이려면 이펙트·무기 GameObject에 **KGBoneBoneFollower**를 붙이고 Target과 Bone Name을 고른다(Follow Rotation을 켜면 회전도 따라간다).

## 지원 범위 (1단계)

| 지원 | 아직 없음 |
|---|---|
| 본(상속 5종), 이미지·메시·가중치 메시·링크드 메시, 메시 변형 키 | 제약(IK, 트랜스폼, 패스, 물리, 슬라이더) — 2단계 |
| 슬롯 색·어두운 색, 첨부물 키, 그리기 순서 키, 블렌드 4종 | 클리핑, 시퀀스 — 3단계 |
| 스킨 1개 + 스킨 전용 본, 이벤트, 선형·계단·베지어 곡선 | 애니메이션 섞기·전환, 여러 트랙, 실행 중 스킨 바꾸기, 본 덮어쓰기, Animator 연동 — 3단계 |

## 개발

- 계산부 `Runtime/Core`는 유니티 없이 컴파일되는 순수 C#(C# 9)이다. 저장소 루트에서 `npm run unity:test`(dotnet test)가 `Tests/Golden`의 정답과 비교한다.
- 정답은 `npm run unity:golden`이 KGBone 핵심(kgbone.wasm)으로 만든다.
- 유니티 EditMode 테스트: `npm run unity:editmode`(확인용 프로젝트 `unity/KGBoneSandbox`).
