# 작업 기록 (LOG)

| 단계 | 내용 | 끝난 시각 (KST) |
|---|---|---|
| 시작 | 작업 시작 | 2026-11-11 20:24 |
| 1단계 | 기능 명세(SPEC, SOURCES, GAPS) | 2026-11-11 20:31 |
| 2단계 | 시험용 파일(samples), FORMAT.md | 2026-11-11 20:34 |
| 3단계 | 화면 명세(UI_SPEC) | 2026-11-11 20:35 |
| 4단계 | 구조 설계: 공통 데이터 모델, 명령 목록(interface/commands.json), Rust↔화면 연결 인터페이스 확정 | 2026-11-11 21:06 |
| 5단계 | 규칙집(CLAUDE.md) | 2026-11-11 21:09 |

## 환경 메모
- 2026-11-11 20:50: 이 PC는 Windows **스마트 앱 컨트롤**(VerifiedAndReputablePolicyState=1)이 켜져 있어, 새로 빌드한 서명 없는 실행 파일(Rust 테스트 .exe, 빌드 스크립트)이 "애플리케이션 제어 정책에서 이 파일을 차단했습니다(os error 4551)"로 막힌다. 보안 설정은 바꾸지 않았다.
  - 대응: 핵심 모듈(kgbone-core)을 외부 의존성 없이(빌드 스크립트 없이) 만들고, 테스트는 wasm32-wasip1로 빌드해 Node WASI로 실행한다(tools/wasi-run.mjs).
  - 화면용 WebAssembly는 wasm-bindgen 없이 C ABI로 내보낸다(kgbone-wasm).
  - Tauri 앱(.exe)은 빌드 스크립트가 필요해 이 PC에서 빌드할 수 없다 → GitHub Actions(Windows)에서 빌드한다(9단계).
- esotericsoftware.com 접속 불가(ECONNRESET) → 검색 요약으로 근거 확인(SOURCES.md).
