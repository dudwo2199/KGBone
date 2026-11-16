# KGBone 기능 명세 (SPEC)

근거: `docs/SOURCES.md`의 S1~S14. 각 기능 줄 끝의 **완료 기준**은 자동 테스트로 확인한다(테스트 이름에 기능 번호를 넣는다. 예: `f_b01_bone_create`).
테스트 위치 약어: **R** = Rust 테스트(`cargo test`), **U** = 화면 단위 테스트(`vitest`), **E** = 화면 흐름 테스트(Playwright), **C** = 유니티 런타임 계산부 테스트(`dotnet test`, 같은 테스트가 유니티 EditMode에서도 돈다), **Y** = 유니티 EditMode 테스트(`npm run unity:editmode`).

## A. 모드
| 번호 | 기능 | 완료 기준 |
|---|---|---|
| A01 | 셋업 모드 / 애니메이트 모드 전환 (`mode.setup`, `mode.animate`, `mode.toggle`) | R: 애니메이트 모드에서 본을 움직이면 셋업 값은 그대로이고 현재 애니메이션에 키가 생긴다. 셋업 모드에서는 셋업 값이 바뀐다 |

## B. 본
| 번호 | 기능 | 완료 기준 |
|---|---|---|
| B01 | 본 생성 (`bone.create`) | R: 부모 아래 새 본이 생기고 이름이 겹치지 않는다(`bone`, `bone2`…). 되돌리면 사라진다 |
| B02 | 본 삭제 (`bone.delete`) | R: 본과 자식 본, 그 본을 쓰는 슬롯·제약·키가 함께 지워지고 되돌리면 모두 복구된다 |
| B03 | 부모 변경 (`bone.setParent`) | R: 부모를 바꿔도 월드 위치가 유지되도록 로컬 값이 다시 계산된다(오차 1e-3 이내). 자기 자손을 부모로 지정하면 거부 |
| B04 | 이동·회전·크기·기울기 (`bone.setTransform`) | R: 손 계산 정답과 월드 위치·회전 비교(오차 1e-3) |
| B05 | 상속 설정 (`bone.setInherit`: normal, onlyTranslation, noRotationOrReflection, noScale, noScaleOrReflection) | R: 상속 종류마다 손 계산 정답과 비교 |
| B06 | 본 색상·길이 (`bone.setColor`, `bone.setLength`) | R: 값이 저장되고 .kgbone 다시 열기 후 같다 |
| B07 | 이름 바꾸기(한글 포함) (`item.rename`) | R: 한글 이름 저장·다시 열기 후 같다. E: 한글 이름 입력 시 자모가 분리되지 않는다 |

## C. 슬롯
| 번호 | 기능 | 완료 기준 |
|---|---|---|
| C01 | 슬롯 생성·삭제 (`slot.create`, `slot.delete`) | R: 생성·삭제·되돌리기 |
| C02 | 색상 (`slot.setColor`) | R: RGBA 저장·Spine JSON `color` 왕복 |
| C03 | 어두운 색 / tint black (`slot.setDarkColor`) | R: `dark` 값 왕복, 끄면 필드가 없어진다 |
| C04 | 블렌드 모드 (`slot.setBlend`: normal, additive, multiply, screen) | R: 왕복. U: 렌더러가 모드별 블렌드 함수를 고른다 |
| C05 | 그리기 순서 (`slot.reorder`) | R: 순서 이동·되돌리기. E: 트리에서 드래그로 순서 변경 |

## D. 첨부물
| 번호 | 기능 | 완료 기준 |
|---|---|---|
| D01 | 이미지(region) (`attachment.create` type=region) | R: 위치·회전·크기·폭·높이 왕복, 정점 4개 월드 좌표 손 계산 일치 |
| D02 | 메시 정점·삼각형 편집 (`mesh.setVertices`, `mesh.setTriangles`) | R: 편집 후 삼각형 인덱스가 정점 수를 넘지 않음, 왕복 |
| D03 | 메시 자동 생성 (`mesh.generate`) | R: 알파 이미지에서 외곽 다각형 + 삼각분할, 모든 삼각형 넓이 > 0, 불투명 픽셀 95% 이상 덮음 |
| D04 | 가중치 바인딩 (`mesh.bind`) | R: 바인딩 후 셋업 포즈의 정점 월드 위치가 바인딩 전과 같다(1e-3) |
| D05 | 가중치 페인팅 (`weights.paint`) | R: 붓질 후 정점마다 가중치 합 = 1(1e-4) |
| D06 | 자동 가중치 (`weights.auto`) | R: 가까운 본의 가중치가 더 크고 합 = 1 |
| D07 | 가중치 메시 정점 위치 | R: 손 계산 정답과 비교(1e-3) |
| D08 | 링크드 메시 (`attachment.create` type=linkedmesh) | R: 부모 메시의 정점·삼각형을 공유, 변형 상속(timelines) 옵션 왕복 |
| D09 | 바운딩 박스 | R: 정점 왕복 |
| D10 | 포인트 | R: 위치·회전 왕복, 월드 위치 손 계산 일치 |
| D11 | 클리핑 (반전·볼록 옵션) | R: `end` 슬롯·반전·볼록 저장, 클리핑 계산 결과 삼각형이 클립 다각형 안(반전이면 밖)에 있다 |
| D12 | 패스 (closed, constantSpeed, lengths) | R: 왕복 |
| D13 | 시퀀스(프레임 넘김) | R: count/start/digits/setup 왕복 |
| D14 | 뷰포트에서 이미지 첨부물 끌어 옮기기(이동 도구, 셋업 모드, `attachment.set`의 x·y) | U: 월드 이동량이 본 로컬로 바뀌고 Shift면 한 축으로만, 끌기 한 번 = 되돌리기 한 번. E: 그림을 끌면 첨부물만 옮겨지고 본은 그대로, 되돌리기 한 번에 원래대로 |

## E. 스킨
| 번호 | 기능 | 완료 기준 |
|---|---|---|
| E01 | 스킨 생성·삭제·전환 (`skin.create`, `skin.delete`, `skin.setActive`) | R: 전환 시 스킨 첨부물이 기본 스킨보다 우선 |
| E02 | 스킨별 첨부물 | R: 같은 슬롯·이름에 스킨마다 다른 첨부물 |
| E03 | 스킨별 본/제약 (`skin.addItem`) | R: 스킨 전용 본은 그 스킨이 켜졌을 때만 계산 |

## F. 제약
| 번호 | 기능 | 완료 기준 |
|---|---|---|
| F01 | IK 1본·2본 (굽힘 방향, mix, softness, compress, stretch, uniform) | R: 손 계산 정답(2본 코사인 법칙)과 비교 1e-3 |
| F02 | IK 부피 보존(4.3, scaleY) | R: 늘어난 비율 s에 대해 scaleY = 1/s (손 계산) |
| F03 | 트랜스폼 제약 4.3 (속성 매핑 from→to, 오프셋·배율, 범위 제한 clamp, 로컬/월드) | R: 이동X→회전 매핑, clamp 적용, 로컬/월드 각각 손 계산 일치 |
| F04 | 패스 제약 (위치·간격·회전 모드: tangent, chain, chainScale / 간격: length, fixed, percent, proportional) | R: 직선 패스 위 본 위치 손 계산 일치 |
| F05 | 물리 제약 (관성·강도·감쇠·질량·바람·중력·mix, 부피 보존) | R: 고정 시간 간격 1/60s로 120단계, 같은 입력 두 번 돌리면 결과 비트 단위 동일. 정지 상태면 움직이지 않음 |
| F06 | 슬라이더 제약(4.3) (애니메이션, 시간, 반복, 더하기, 본+속성 매핑, mix) | R: 슬라이더 시간 t에서 결과 = 애니메이션을 t에 적용한 결과 × mix. .kgbone 왕복 |
| F07 | 제약 순서 (`constraint.reorder`) | R: 순서대로 적용, 왕복 |

## G. 애니메이션
| 번호 | 기능 | 완료 기준 |
|---|---|---|
| G01 | 애니메이션 생성·삭제·이름 변경 | R: 왕복·되돌리기 |
| G02 | 본 변환 키 (rotate, translate, scale, shear, 분리 X/Y, inherit) | R: 키 사이 값 손 계산 일치 |
| G03 | 슬롯 색상 키(rgba, rgb, alpha, rgba2, rgb2), 첨부물 전환 키 | R: 왕복, 시점 값 일치 |
| G04 | 그리기 순서 키 | R: 시점별 순서 일치 |
| G05 | 메시 변형(deform) 키 | R: 중간 시점 정점 = 선형 보간 |
| G06 | 제약 값 키 (ik, transform, path, physics, slider) | R: 시점 값 일치 |
| G07 | 이벤트 키 | R: 구간 [t0, t1) 안의 이벤트만 발생 |
| G08 | 보간: 선형 / 계단 / 베지어 | R: 베지어 (0.25,0,0.75,1)의 t=0.5 값 = 0.5, 계단은 다음 키 전까지 앞 값, 손 계산 일치 |
| G09 | 도프시트 (키 선택·이동·복사·삭제) | R: `key.move`, `key.delete` 되돌리기. E: 키 표시 |
| G10 | 그래프(곡선) 편집기 | R: `key.setCurve` 왕복. U: 곡선 점 좌표 변환 |
| G11 | 재생·반복·속도 | U: 속도 2배면 같은 실제 시간에 2배 진행, 반복 끄면 끝에서 멈춤 |
| G12 | 오니언 스킨 | U: 앞뒤 프레임 수 설정에 맞는 시점 목록 |
| G13 | 바뀐 값 키 지정(K), 제약 적용 후 값 키(Key Constrained, 4.3) | R: 키가 생긴다 |

## H. 이벤트
| 번호 | 기능 | 완료 기준 |
|---|---|---|
| H01 | 이벤트 생성 (int, float, string, audio, volume, balance) | R: 왕복(한글 문자열 포함) |
| H02 | 이벤트 키 지정 | R: 키 값이 기본값과 다를 때만 저장 |

## I. 입력
| 번호 | 기능 | 완료 기준 |
|---|---|---|
| I01 | 숫자 입력칸 수식: `v`/`value` 현재 값, `+=`, `-=`, `*=`, `/=`, 사칙연산·괄호 | R: `v*2`, `+=5`, `(1+2)*3` 등 정답 일치, 잘못된 식은 오류 |
| I02 | 한국어 입력 조합 중 단축키 무시 | U: `isComposing` 중 키는 명령으로 가지 않는다 |
| I03 | 되돌리기/다시 실행 (Ctrl+Z, Ctrl+Y, Ctrl+Shift+Z), 드래그 한 번 = 되돌리기 한 번 | R: `history.begin/end` 묶음이 한 번에 되돌려진다 |

## J. 파일
| 번호 | 기능 | 완료 기준 |
|---|---|---|
| J01 | .kgbone 저장·열기 | R: samples 전체 열기→저장→다시 열기 내용 같음 |
| J02 | 편집 없이 저장하면 바이트 동일 | R: 모든 .kgbone 샘플 바이트 비교 |
| J03 | 편집 후 저장 시 나머지 키 순서·숫자 표기 유지 | R: 본 하나 이름 바꾼 뒤 다른 줄이 그대로 |
| J04 | Spine JSON 가져오기(4.3 우선, 4.2 읽기) | R: 4.2/4.3 샘플 읽기 성공 |
| J05 | Spine JSON 내보내기 — 모르는 필드·키 순서·숫자 표기 보존 | R: 가져온 뒤 바로 내보내면 의미가 같고, 모르는 필드가 남고, 키 순서가 같다 |
| J06 | .atlas 읽기·쓰기 | R: 샘플 atlas 왕복 |
| J07 | 형식 버전 자동 변환 | R: formatVersion 0 파일을 열면 1로 바뀐다 |
| J08 | 이미지 폴더 불러오기·자동 갱신 | R(앱): 폴더 PNG 목록. U: 갱신 이벤트 후 목록 갱신 |
| J09 | 실행 인자로 파일 열기, 파일 연결 등록 | R(앱): 인자 해석 함수 테스트 |
| J10 | 저장하지 않은 변경이 있으면 닫을 때 확인 | U: dirty일 때 확인 창 요청 |

## K. 내보내기
| 번호 | 기능 | 완료 기준 |
|---|---|---|
| K01 | Spine JSON + .atlas + PNG (텍스처 패킹) | R: 영역이 겹치지 않고 페이지 안, atlas가 다시 읽힘 |
| K02 | PNG 시퀀스 | R: 프레임 수 = 길이×fps(+1), 각 PNG 크기 일치, 본 이미지 픽셀이 그려짐 |
| K03 | 4.3 미확인 표기 경고 | R: 슬라이더·4.2로 표현 불가한 트랜스폼 제약이 있으면 경고 목록에 나온다 |

## L. 화면 흐름 (E)
| 번호 | 기능 | 완료 기준 |
|---|---|---|
| L01 | 새 파일 → 본 추가 → 이미지 붙이기 → 키 지정 → 재생 → 저장 → 다시 열기 | E: Playwright로 끝까지 통과 |
| L02 | 메뉴·단축키·버튼이 같은 명령 호출 | U: 명령 목록의 모든 단축키가 등록된 명령 id를 가리킨다 |

## M. 유니티 런타임 (1단계: 재생 + 본 위치 읽기)
설계: `docs/superpowers/specs/2026-11-16-unity-runtime-stage1-design.md`. 위치: `unity/com.kgbone.runtime/`. 계산 결과는 KGBone 핵심과 같아야 한다(정답 파일: `npm run unity:golden`).
| 번호 | 기능 | 완료 기준 |
|---|---|---|
| M01 | UPM 패키지 구조(Unity 6000.0+, URP), 계산부는 유니티 없이 컴파일 | C: `m01_world_bone_identity_maps_point`. Y: 확인용 프로젝트에서 패키지 전체 컴파일 |
| M02 | JSON·`.kgbone` 읽기(기본값, 키·곡선, 시퀀스 이미지 이름) | C: `m02_json_*`, `m02_loader_*` |
| M03 | 본 월드 계산(상속 5종, 반사 부모) | C: `m03_inherit_modes_hand_computed`, `m03_reflected_parent_no_scale_keeps_reflection_only_when_asked` |
| M04 | 곡선(선형·계단·베지어, deform 비율 곡선) | C: `m04_*` |
| M05 | 본 키(셋업 대비 더하기·곱하기, 축별 이름, 상속 키) | C: `m05_bone_keys_relative_to_setup` |
| M06 | 슬롯 키(색·어두운 색·첨부물)와 그리기 순서 키 | C: `m06_*` |
| M07 | 그리기 목록(region, 메시, 가중치 메시, 링크드 메시, deform, 블렌드)이 핵심과 같음 | C: `m07_all_golden_cases_match_core`(정답 64개), `m07_region_corners_hand_computed` |
| M08 | 스킨과 스킨 전용 본 | C: `m08_skin_required_bone_inactive_without_skin` + 정답의 스킨별 케이스 |
| M09 | 이벤트([last, time), 반복 바퀴, 시간은 키에 적힌 시간)와 반복 시점 | C: `m09_*` |
| M10 | 가져오기 도구(데이터 에셋, 묶은 텍스처, 블렌드별 머티리얼, 미지원 기능·없는 이미지 경고) | Y: `m10_*` |
| M11 | 재생 컴포넌트(재생·정지·속도·반복·이벤트·끝·뒤집기·Sorting, 다시 가져오면 에디터 모드에서도 다시 그림) | Y: `m11_*` |
| M12 | 본 위치 읽기와 본 따라가기 | Y: `m12_try_get_bone_with_flip`, `m12_follower_tracks_bone` |

## 이번에 제외 (→ docs/GAPS.md)
바이너리 .skel 입출력, PSD 직접 관리, HTML/웹 컴포넌트 내보내기, 스킨별 아틀라스 패킹, 프로젝트 ZIP 패키징, GIF/동영상 내보내기, 문제(Issues) 뷰 자동 해결.
