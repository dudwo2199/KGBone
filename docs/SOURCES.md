# 근거 문서 (SOURCES)

KGBone은 클린룸 방식으로 만든다. 아래 표에 적힌 공개 문서만 근거로 쓴다.
원본 프로그램 코드, `EsotericSoftware/spine-runtimes` 저장소의 모든 파일(코드·문서·CHANGELOG), 공식 런타임, 공식 예제 파일·이미지는 열지도, 쓰지도 않았다.

## 접속 상황 (2026-11-11 확인)

이 PC에서 `esotericsoftware.com`(및 `en.` 하위 도메인)은 HTTPS 연결이 즉시 끊겼다(`ECONNRESET`, curl·내장 브라우저 모두). 웹 보관소(web.archive.org)도 접속되지 않았다.
그래서 각 공식 문서의 내용은 **검색 엔진이 색인한 해당 공식 페이지의 요약**으로 확인했고, 페이지 본문을 직접 읽지는 못했다.
직접 확인하지 못해 표기가 불확실한 항목은 `docs/GAPS.md`에 "Spine 4.3 JSON 호환 미확인"으로 적었다.
접속이 가능해지면 아래 URL을 다시 열어 SPEC/FORMAT을 대조해야 한다(GAPS.md 항목 G-01).

## 문서 목록

| # | URL | 확인 날짜 | 확인 방법 | 무엇의 근거로 썼나 |
|---|---|---|---|---|
| S1 | https://esotericsoftware.com/spine-json-format | 2026-11-11 | 검색 요약 | Spine JSON 가져오기·내보내기 전체 구조(skeleton/bones/slots/ik/transform/path/physics/skins/attachments/events/animations), 기본값 생략 규칙, 커브 표기. 색인된 본문 예제는 3.8대 버전 표기로, **4.3 기준으로 갱신됐는지 확인 불가** |
| S2 | https://esotericsoftware.com/spine-atlas-format | 2026-11-11 | 검색 요약 | .atlas 읽기·쓰기: 페이지 머리(size, format, filter, repeat, pma), 영역(bounds, offsets, rotate, index, split, pad), offsets 계산식 |
| S3 | https://esotericsoftware.com/blog/Spine-4.3-released | 2026-11-11 | 검색 요약 | 4.3 신기능: 트랜스폼 제약 개편(속성 간 매핑, 범위 제한, 월드/로컬 공간), 슬라이더, IK·물리의 부피 보존(scaleY), "Key Constrained" 단축키 |
| S4 | https://en.esotericsoftware.com/blog/The-4.3-beta-is-now-available | 2026-11-11 | 검색 요약 | S3과 같은 트랜스폼 제약 변경 확인, JSON 가독성·크기·정밀도 개선 언급 |
| S5 | https://esotericsoftware.com/spine-changelog | 2026-11-11 | 검색 요약 | 4.3.x 패치에서 트랜스폼 제약 JSON 내보내기(mixY, mixScaleY) 수정 이력 → 4.3 표기가 4.2와 다를 수 있다는 근거 |
| S6 | https://en.esotericsoftware.com/spine-sliders | 2026-11-11 | 검색 요약 | 슬라이더 속성: 애니메이션, 반복(loop), 더하기(additive), 프레임, 본+속성 매핑, 혼합(mix, 키 가능) |
| S7 | http://esotericsoftware.com/spine-transform-constraints | 2026-11-11 | 검색 요약 | 트랜스폼 제약 속성: 대상, 혼합, 오프셋, 로컬/상대, 4.3 매핑·범위 |
| S8 | http://esotericsoftware.com/spine-cheat-sheet | 2026-11-11 | 검색 요약 | 단축키: 도구 전환 키, P(부모 지정), +/-(그리기 순서), K(바뀐 값 키), 재생 키 |
| S9 | https://us.esotericsoftware.com/spine-tools | 2026-11-11 | 검색 요약 | 도구 목록(포즈·생성·가중치·회전·이동·크기·기울기), 오른쪽 클릭으로 직전 도구 전환 |
| S10 | http://en.esotericsoftware.com/spine-ui | 2026-11-11 | 검색 요약 | 화면 구성(뷰포트, 메인 도구 모음, 트리, 도프시트, 그래프 등) |
| S11 | http://esotericsoftware.com/spine-tree / spine-dopesheet / spine-graph / spine-keys / spine-slots | 2026-11-11 | 검색 요약 | 트리 뷰, 도프시트, 그래프, 키 지정, 슬롯(색상·어두운 색·블렌드) |
| S12 | http://esotericsoftware.com/spine-texture-packer | 2026-11-11 | 검색 요약 | 텍스처 패킹 옵션(공백 제거, 회전, 페이지 크기 제한) |
| S13 | http://esotericsoftware.com/spine-export | 2026-11-11 | 검색 요약 | 내보내기 옵션(JSON, 보기 좋게 출력, 버전, 비필수 데이터) |
| S14 | http://esotericsoftware.com/spine-physics-constraints · spine-ik-constraints · spine-path-constraints | 2026-11-11 | 검색 요약 | 물리(관성·강도·감쇠·질량·바람·중력), IK(굽힘 방향·늘이기·압축·균등), 패스(위치·간격·회전 모드) 속성 이름 |

## 문서에서 정하지 못해 직접 설계한 것

- `.kgbone` 형식 전체 (→ `docs/FORMAT.md`)
- 4.3 트랜스폼 제약·슬라이더의 Spine JSON 키 이름: 문서 본문 확인 불가 → 내보내기 시 경고, 4.2 표기로 낼 수 있는 경우만 4.2 표기로 냄 (→ `docs/GAPS.md`)
- 물리 계산식, 자동 가중치, 자동 메시 생성 알고리즘: 일반적인 수치 기법(고정 간격 적분, 거리 반비례 가중치, 알파 경계 다각형 + 귀 자르기 삼각분할)으로 직접 작성
