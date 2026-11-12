# .kgbone 파일 형식 (formatVersion 1)

KGBone의 기본 프로젝트 파일. 누구나 읽고 쓸 수 있도록 공개한다.

## 1. 기본 규칙
- UTF-8 텍스트 JSON 파일 하나. BOM 없음. 줄바꿈 `\n`.
- 이미지는 파일 안에 넣지 않는다. `imagesPath`(이 파일 기준 상대 경로) 폴더의 PNG를 첨부물 `path`(확장자 뺀 상대 경로, `/` 구분)로 참조한다.
- 숫자는 JSON 숫자. 각도는 도(degree), 반시계 방향이 +. 좌표계는 y가 위로 증가.
- 색은 `"RRGGBBAA"` 16진 문자열(어두운 색은 `"RRGGBB"`).
- 기본값과 같은 필드는 생략할 수 있다. 읽는 쪽은 생략된 필드를 아래 표의 기본값으로 본다.
- 모르는 필드는 읽을 때 보존하고 쓸 때 그대로 다시 쓴다(앞으로의 버전 호환).

### 표준 출력 모양 (KGBone이 쓰는 방식)
- 들여쓰기 탭 1개, 키 뒤 `": "`.
- 원소가 모두 숫자·문자열·참거짓·null인 배열은 한 줄(`[1, 2, 3]`).
- 그 밖의 객체·배열은 원소마다 한 줄.
- 편집하지 않고 저장하면 원래 바이트를 그대로 쓴다. 편집했으면 바뀐 값만 바꾸고, 나머지 키 순서와 숫자 표기(예: `1.50`)는 원본을 유지한다.

## 2. 최상위
```json
{
	"formatVersion": 1,
	"generator": "KGBone 0.1.0",
	"imagesPath": "./images",
	"skeleton": { ... },
	"editor": { ... }
}
```
| 키 | 형 | 설명 |
|---|---|---|
| formatVersion | 정수 | 형식 버전. 현재 1. 0 또는 없음 = 초기 시험판(아래 7장 변환 규칙) |
| generator | 문자열 | 만든 프로그램(정보용) |
| imagesPath | 문자열 | 이미지 폴더 상대 경로. 기본 `"./images"` |
| skeleton | 객체 | 뼈대 데이터(3장) |
| editor | 객체 | 작업 상태(6장). 없어도 열린다 |

## 3. skeleton
| 키 | 형 | 기본값 | 설명 |
|---|---|---|---|
| x, y, width, height | 수 | 0 | 셋업 포즈 경계 상자(정보용) |
| fps | 수 | 30 | 도프시트 프레임 단위 |
| referenceScale | 수 | 100 | 물리 단위 기준 |
| bones | 배열 | [] | 본. **부모가 자식보다 앞**에 온다. 첫 본이 루트 |
| slots | 배열 | [] | 슬롯. 배열 순서 = 셋업 그리기 순서(앞이 뒤에 그려짐) |
| constraints | 배열 | [] | 모든 제약. **배열 순서 = 적용 순서** |
| skins | 배열 | [] | 스킨. 이름이 `default`인 것이 기본 스킨 |
| events | 배열 | [] | 이벤트 정의 |
| animations | 배열 | [] | 애니메이션 |

### 3.1 bone
| 키 | 기본값 | 설명 |
|---|---|---|
| name | (필수) | 고유 이름 |
| parent | 없음 | 부모 본 이름 |
| length | 0 | 길이 |
| x, y | 0 | 부모 기준 위치 |
| rotation | 0 | 회전 |
| scaleX, scaleY | 1 | 크기 |
| shearX, shearY | 0 | 기울기 |
| inherit | `"normal"` | `normal`, `onlyTranslation`, `noRotationOrReflection`, `noScale`, `noScaleOrReflection` |
| skinRequired | false | true면 이 본을 포함한 스킨이 켜졌을 때만 계산 |
| color | `"989898FF"` | 에디터 표시 색 |
| icon | 없음 | 에디터 아이콘 이름 |
| visible | true | 에디터 표시 여부 |

### 3.2 slot
| 키 | 기본값 | 설명 |
|---|---|---|
| name | (필수) | |
| bone | (필수) | 붙은 본 |
| color | `"FFFFFFFF"` | |
| dark | 없음 | 어두운 색(tint black). 있으면 켜짐 |
| attachment | 없음 | 셋업 포즈에서 보이는 첨부물 이름 |
| blend | `"normal"` | `normal`, `additive`, `multiply`, `screen` |
| visible | true | 에디터 표시 여부 |

### 3.3 constraints
모든 제약은 `type`, `name`, `skinRequired`(기본 false)를 가진다.

**IK** (`"type": "ik"`)
| 키 | 기본값 | 설명 |
|---|---|---|
| bones | (필수) | 1개 또는 2개 |
| target | (필수) | 목표 본 |
| mix | 1 | 혼합 |
| softness | 0 | 부드러움(2본) |
| bendPositive | true | 굽힘 방향 |
| compress | false | 짧아지기 허용(1본) |
| stretch | false | 늘어나기 허용 |
| uniform | false | 늘어날 때 Y도 같이 |
| volume | false | 부피 보존(4.3): 늘어난 비율 s에 대해 scaleY에 1/s를 곱한다 |

**트랜스폼** (`"type": "transform"`) — 4.3 속성 매핑 모델
| 키 | 기본값 | 설명 |
|---|---|---|
| bones | (필수) | 제약을 받는 본들 |
| source | (필수) | 값을 읽어 올 본 |
| localSource | false | true면 source의 로컬 값, false면 월드 값 |
| localTarget | false | true면 대상 본의 로컬 값을, false면 월드 값을 바꾼다 |
| additive | false | true면 대상 현재 값에 더한다 |
| clamp | false | true면 결과를 to의 [offset, max] 범위로 제한 |
| mixRotate, mixX, mixY, mixScaleX, mixScaleY, mixShearY | 1 | 받는 속성별 혼합 |
| properties | 아래 | 매핑 목록 |

`properties`: `[{ "from": 속성, "offset": 수, "to": [{ "property": 속성, "offset": 수, "max": 수, "scale": 수 }] }]`
속성 이름은 `rotate`, `x`, `y`, `scaleX`, `scaleY`, `shearY`.
계산: `d = source.from − from.offset`, `값 = to.offset + d × to.scale`, clamp면 `값`을 `to.offset`과 `to.max` 사이로 자른다.
비가산이면 `대상 = 대상 + (값 − 대상) × mix`, 가산이면 `대상 = 대상 + 값 × mix`.
`properties`가 없으면 6개 속성이 각자 같은 속성으로 이어진 기본 매핑(`offset 0`, `scale 1`, `max 0`)이다(4.2 방식과 같은 결과).
4.2 JSON의 `offsetRotation` 등은 같은 속성 매핑의 `to.offset`으로 옮긴다.

**패스** (`"type": "path"`)
| 키 | 기본값 |
|---|---|
| bones, target(슬롯) | (필수) |
| positionMode | `"percent"` (`fixed`) |
| spacingMode | `"length"` (`fixed`, `percent`, `proportional`) |
| rotateMode | `"tangent"` (`chain`, `chainScale`) |
| offsetRotation, position, spacing | 0 |
| mixRotate, mixX, mixY | 1 |

**물리** (`"type": "physics"`)
| 키 | 기본값 | 설명 |
|---|---|---|
| bone | (필수) | |
| x, y, rotate, scaleX, shearX | 0 | 속성별 영향 비율(0~1) |
| limit | 5000 | 속도 제한 |
| fps | 60 | 고정 계산 간격(초당 단계 수) |
| inertia | 1 | 관성 |
| strength | 100 | 복원 강도 |
| damping | 1 | 감쇠 |
| mass | 1 | 질량 |
| wind, gravity | 0 | |
| mix | 1 | |
| volume | false | 부피 보존(4.3): scaleX가 s배면 scaleY를 1/s배 |

**슬라이더** (`"type": "slider"`, 4.3)
| 키 | 기본값 | 설명 |
|---|---|---|
| animation | (필수) | 적용할 애니메이션 이름 |
| time | 0 | 셋업 시점(초) |
| loop | false | 애니메이션 길이 밖에서 반복 |
| additive | false | 현재 포즈에 더하기 |
| mix | 1 | 영향 비율 |
| bone | 없음 | 있으면 이 본의 `property` 값으로 시점을 정함 |
| property | `"x"` | `rotate`, `x`, `y`, `scaleX`, `scaleY`, `shearY` |
| propertyFrom, propertyTo | 0, 1 | 본 속성 값 범위. 이 범위가 시점 0 ~ 애니메이션 길이에 대응 |
| local | false | 본 값을 로컬로 읽기 |

### 3.4 skin
```json
{ "name": "default", "bones": [], "constraints": [], "attachments": { "슬롯이름": { "첨부물이름": { ... } } } }
```

### 3.5 attachment
모든 첨부물은 `type`과 선택 `name`(실제 이미지 이름이 키와 다를 때)을 가진다. `path`가 없으면 키 이름이 이미지 경로다.

| type | 키 (기본값) |
|---|---|
| region | path, x, y (0), rotation (0), scaleX, scaleY (1), width, height (32), color (`FFFFFFFF`), sequence |
| mesh | path, width, height, color, uvs, triangles, vertices, weighted (false), hull (0), edges, sequence |
| linkedmesh | path, width, height, color, parent(필수), skin (없음=기본 스킨), timelines (true) |
| boundingbox | vertexCount, vertices, weighted, color (`60F000FF`) |
| point | x, y, rotation (0), color (`F1F100FF`) |
| clipping | end(슬롯 이름), vertexCount, vertices, weighted, inverse (false), convex (false), color (`CE3A3AFF`) |
| path | closed (false), constantSpeed (true), vertexCount, vertices, weighted, lengths, color (`FF7F00FF`) |

`vertices` 표기:
- `weighted: false` → `[x0, y0, x1, y1, …]` (슬롯 본 기준)
- `weighted: true` → 정점마다 `[본 수 n, (본 번호, x, y, 가중치) × n]`을 이어 붙인다. 본 번호는 `skeleton.bones` 배열 위치.

`uvs`는 0~1 텍스처 좌표 `[u0, v0, …]`, `triangles`는 정점 번호 3개씩.
`sequence`: `{ "count": 1, "start": 1, "digits": 0, "setup": 0 }`.

### 3.6 event
`{ "name", "int": 0, "float": 0, "string": "", "audio": "", "volume": 1, "balance": 0 }`

### 3.7 animation
```json
{
	"name": "걷기",
	"bones": { "본": { "rotate": [키...], "translate": [...] } },
	"slots": { "슬롯": { "rgba": [...], "attachment": [...] } },
	"constraints": { "제약": { "mix": [...] } },
	"deform": { "스킨": { "슬롯": { "첨부물": [...] } } },
	"sequence": { "스킨": { "슬롯": { "첨부물": [...] } } },
	"drawOrder": [{ "time": 0, "offsets": [{ "slot": "이름", "offset": 1 }] }],
	"events": [{ "time": 0, "name": "발소리", "int": 0, "float": 0, "string": "" }]
}
```
**키** 공통 모양: `{ "time": 초, "v": [값...], "s": 문자열, "curve": 곡선 }`
- `v` 생략 = 빈 배열, `s`는 문자열 값이 필요한 타임라인만(첨부물 이름, 상속 종류, 시퀀스 모드). 첨부물 키에서 `s`가 없으면 "비움".
- deform 키는 `offset`(정수, 기본 0)을 더 가진다.
- `curve` 생략 = 선형, `"stepped"` = 계단, 숫자 배열 = 베지어. 베지어는 값 채널마다 `[cx1, cy1, cx2, cy2]`를 이어 붙인다. cx는 시간(초), cy는 값, 모두 절대 좌표.

**값의 기준**: 본 타임라인 값은 셋업 값에서의 차이다 — rotate·translate·shear는 셋업 값에 **더하고**, scale은 셋업 값에 **곱한다**. 슬롯 색·제약 값은 절대값. deform은 셋업 정점에 더할 차이값.
제약 타임라인 이름 `""`(빈 이름)의 물리 키는 모든 물리 제약에 적용된다(Spine JSON의 전체 물리 키와 같은 뜻).

| 대상 | 타임라인 | v의 뜻 |
|---|---|---|
| 본 | rotate | [각도] |
| 본 | translate, scale, shear | [x, y] |
| 본 | translatex, translatey, scalex, scaley, shearx, sheary | [값] |
| 본 | inherit | s = 상속 종류 (계단) |
| 슬롯 | attachment | s = 첨부물 이름, 없으면 비움 (계단) |
| 슬롯 | rgba / rgb / alpha | [r,g,b,a] / [r,g,b] / [a] (0~1) |
| 슬롯 | rgba2 / rgb2 | [r,g,b,a, dr,dg,db] / [r,g,b, dr,dg,db] |
| 제약(ik) | ik | [mix, softness, bendPositive(1/−1), compress(0/1), stretch(0/1)] |
| 제약(transform) | transform | [mixRotate, mixX, mixY, mixScaleX, mixScaleY, mixShearY] |
| 제약(path) | position / spacing / mix | [값] / [값] / [mixRotate, mixX, mixY] |
| 제약(physics) | inertia, strength, damping, mass, wind, gravity, mix | [값] |
| 제약(physics) | reset | [] (그 시점에 물리 초기화) |
| 제약(slider) | time / mix | [값] |
| deform | (배열 자체) | `offset`(정수, 기본 0) + v = 셋업 정점에서의 차이값 |
| sequence | (배열 자체) | s = `hold`, `once`, `loop`, `pingpong`, `onceReverse`, `loopReverse`, `pingpongReverse`, v = [index, delay] |

## 6. editor (작업 상태, 뼈대 결과에는 영향 없음)
| 키 | 기본값 | 설명 |
|---|---|---|
| mode | `"setup"` | `setup` / `animate` |
| view | `{ "x": 0, "y": 0, "zoom": 1 }` | 뷰포트 중심과 확대 비율 |
| activeAnimation | 없음 | |
| activeSkin | 없음 | |
| time | 0 | 현재 시점 |
| treeExpanded | [] | 펼친 트리 노드 id 목록 |
| onion | `{ "enabled": false, "before": 2, "after": 2, "step": 1 }` | 오니언 스킨 |
| playback | `{ "speed": 1, "loop": true }` | |

## 7. 버전 변환
- formatVersion 0(또는 없음): 제약이 `ik`, `transform`, `path`, `physics` 배열로 따로 있고 각자 `order` 숫자를 가진 초기 시험판. 열 때 `order` 순으로 합쳐 `constraints` 배열로 바꾸고 formatVersion을 1로 올린다.

## 8. 계산 규칙 (KGBone 설계, 원본과 결과가 다를 수 있음)
- 이산 값(첨부물·상속·그리기 순서·시퀀스·IK 참거짓 값)은 섞기 비율 ≥ 0.5일 때만 바뀐다. 첫 키 앞은 첫 키 값.
- deform 베지어는 첫 채널 `[cx1, cy1, cx2, cy2]`를 0~1 비율 곡선으로 본다.
- IK 2본: 첫 본의 부모 공간에서 코사인 법칙. softness: d' = (L−s) + s·(1−e^(−(d−(L−s))/s)) (L = 두 본 길이 합). stretch: 부모 scaleX × d/L.
- 트랜스폼: 비가산 회전은 차이를 (−180, 180]으로 감아 섞는다. 월드 값을 로컬로 되돌리면 기울기는 shearY로 표현한다.
- 패스: 구간마다 64등분 길이 표로 등속 매개변수화(`lengths` 필드는 계산에 쓰지 않음). 열린 패스는 범위 밖을 양 끝으로 자르고, 닫힌 패스는 감는다. proportional = spacing × 전체 길이 × (본 길이 / 길이 합).
- 물리: 질점 두 개(본 원점·끝, 길이 0이면 referenceScale 거리 점), 감쇠 c = damping·√(strength·mass), 바람·중력은 가속도 × referenceScale/100, 고정 간격 1/fps.
- 클리핑 반전: 볼록 조각을 변마다 "앞 변들의 안쪽 ∩ 그 변의 바깥쪽"으로 나눠 뺀다.
- 자동 메시: Douglas–Peucker 허용 오차 = 0.75 + (1 − detail)·1.75 px, 외곽선을 tol/2 바깥으로 민다. detail ≥ 0.3이면 안쪽 점 + 들로네식 변 뒤집기.
- 자동 가중치 = 1/(d + 0.01)³(본 선분까지 거리), 정점당 최대 4개, 합 1. 페인팅 감쇠 = 1 − r/radius.
