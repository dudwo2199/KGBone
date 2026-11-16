# KGBoneSandbox

KGBone 유니티 런타임(`../com.kgbone.runtime`)을 손으로 확인하고 EditMode 테스트를 돌리는 유니티 프로젝트.

- `npm run unity:sandbox`: 저장소의 시험 파일(`samples/`)을 `Assets/Samples/`로 복사한다(커밋하지 않음).
- EditMode 테스트(유니티 에디터로 이 프로젝트를 열어 둔 상태면 먼저 닫는다):
  ```
  "C:/Program Files/Unity/Hub/Editor/6000.3.17f1/Editor/Unity.exe" -batchmode -nographics -projectPath unity/KGBoneSandbox -runTests -testPlatform EditMode -testResults unity/KGBoneSandbox/Logs/editmode.xml -logFile -
  ```
