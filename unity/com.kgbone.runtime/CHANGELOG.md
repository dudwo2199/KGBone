# 변경 기록

## 0.3.0 (개발 중)

- 시퀀스: 시퀀스 키(hold, once, loop, pingpong, onceReverse, loopReverse, pingpongReverse, 프레임 간격)로 프레임 이미지를 넘긴다. 가져올 때 모든 프레임 이미지를 묶는다.
- 클리핑: 클리핑 슬롯부터 end 슬롯까지 그려지는 삼각형을 다각형으로 자른다(반전·볼록, 오목 다각형, 가중치·변형 키 클립 다각형).
- 번호 트랙: `SetAnimation`·`AddAnimation`·`ClearTrack`·`AddEmpty`·`SetTrackAlpha`·`GetTrackAnimation`, 섞는 시간(`defaultMix`, 기본 0), 윗 트랙은 키 있는 값만 덮음, `TrackCompleted`. 이벤트(`KGBoneEvent`)에 `Track`·`Animation`.
- `Play`·`Stop`·`Time`·`IsPlaying`·`CurrentAnimation`은 트랙 0 기준으로 지금과 같게 동작한다.
- 클리핑·시퀀스 가져오기 경고를 없앴다. 정답 비교에 `spine43-clipping`, `unity-stage3-clipping`, `unity-stage3-sequence`를 더했다.

## 0.2.0 (개발 중)

- 제약 계산: IK(1본·2본, 부드러움·늘이기·줄이기·부피 보존), 트랜스폼(속성 매핑, 로컬·월드, 가산, 자르기), 슬라이더(본 연결·반복·가산), 패스(열림·닫힘 베지어, 일정 속도, 가중치 패스, 패스 변형 키, 위치 percent·fixed, 간격 length·fixed·percent·proportional, 회전 tangent·chain·chainScale).
- 물리 제약: 고정 간격 계산, reset 키(이름 "" = 모두), 플레이 모드에서 매 프레임 진행(최대 0.1초, 애니메이션이 멈춰도 진행), `ResetPhysics()`.
- 물리 월드 움직임 반영 옵션(`physicsMoveFactor`, `physicsRotateFactor`, 기본 0): GameObject 이동·회전을 흔들림에 반영.
- 핵심과 같은 결과인지 정답 912개(물리 연속 724프레임 포함)로 비교.
- 제약 값 타임라인(ik, transform, slider time·mix 등)과 스킨 전용 제약, 핵심과 같은 제약 적용 순서.
- 계산하는 제약은 가져올 때 경고하지 않는다. 아직 계산하지 않는 종류만 "{종류} 제약 {n}개는 아직 계산하지 않습니다"로 알린다.

## 0.1.0 (배포 전)

- 1단계: `.kgbone` 가져오기 도구, 재생 컴포넌트(KGBoneAnimation), 본 따라가기(KGBoneBoneFollower), URP 셰이더.
- 계산부: 본(상속 5종), 이미지·메시·가중치 메시·링크드 메시, 메시 변형, 슬롯 색·어두운 색·첨부물·그리기 순서 키, 스킨, 이벤트, 곡선.
- KGBone 핵심과 같은 결과인지 정답 64개로 비교.
- `.kgbone`을 다시 가져오면 에디터 모드의 씬에서도 그 데이터를 쓰는 캐릭터를 바로 다시 그린다.
- 코드로 만든 캐릭터(AddComponent 뒤에 데이터·애니메이션 이름을 넣음)도 Play On Awake로 시작하자마자 재생한다(자동 재생을 Start에서 함).
- 에디터 모드에서 스크립트로 데이터·스킨·애니메이션 이름·뒤집기를 바꾸면 다음 에디터 갱신에서 다시 그린다.
