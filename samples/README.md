# 시험용 파일 (samples)

실제 Spine 4.3에서 내보낸 파일은 이 PC에 없으므로, 모두 `tools/gen-samples.mjs`로 **직접 만든** 파일이다.
이미지(`images/*.png`)도 코드로 그렸다. 공식 예제 파일·이미지는 쓰지 않았다.

| 파일 | 담은 기능 |
|---|---|
| spine43/bones-only.json | 본만 (상속 종류, 크기·기울기) |
| spine43/mesh-weights.json | 가중치 메시, 베지어 곡선, 메시 변형 키 |
| spine43/ik.json | 2본·1본 IK, IK 키 |
| spine43/transform.json | 트랜스폼 제약(월드, 로컬+상대) |
| spine43/path.json | 패스 첨부물·패스 제약 |
| spine43/physics.json | 물리 제약, 물리 키·초기화 |
| spine43/clipping.json | 클리핑 |
| spine43/skins.json | 스킨 3개, 스킨 전용 본, 어두운 색, 블렌드 |
| spine43/events.json | 이벤트(한글 문자열), 모르는 필드·숫자 표기(2.50) 보존 시험 |
| spine42/character-4.2.json + .atlas | 4.2 형식 캐릭터(계단 곡선, 그리기 순서, 슬롯 색, 상속 키) |
| kgbone/full-4.3.kgbone | 4.3 신기능 전부(슬라이더, IK·물리 부피 보존, 트랜스폼 속성 매핑) + 한글 이름 |
| kgbone/bones-only.kgbone | 본만 |
| kgbone/notation.kgbone | 키 순서·숫자 표기(1.50, 10.0, 1e2)·모르는 필드 보존 시험 |
| kgbone/legacy-v0.kgbone | formatVersion 0 → 1 자동 변환 시험 |

`spine43/*.json`의 `"spine": "4.3.00"` 파일 구조는 공개 문서의 4.2 구조와 같다(4.3 표기 미확인, docs/GAPS.md G-13).
실제 Spine 4.3 내보내기 파일이 생기면 `samples/real/`에 넣고 구조에 맞춰 고친다.
