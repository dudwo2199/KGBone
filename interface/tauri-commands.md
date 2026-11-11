# 앱 본체(Tauri) 명령

`src-tauri/src/commands/*.rs`의 `#[tauri::command]` 함수. 화면 쪽 짝은 `app/src/platform/tauri.ts`(`Platform` 인터페이스 구현).
뼈대 계산·파일 형식 처리는 모두 `kgbone-core`가 하고, 앱 본체는 디스크·창·레지스트리만 다룬다.

| 명령 | 인자 | 결과 | 설명 |
|---|---|---|---|
| `kg_read_text` | `path` | 문자열 | UTF-8 텍스트 읽기 |
| `kg_write_text` | `path`, `text` | | 쓰기(폴더 자동 생성) |
| `kg_read_binary` | `path` | 바이트(raw 응답) | |
| `kg_write_binary` | `path`, `base64` | | base64 → 바이트 쓰기 |
| `kg_list_images` | `dir` | `[{name, path, width, height}]` | 하위 폴더 포함 PNG 목록. 크기는 `kgbone_core::export::png_size`로 읽음. `name`은 dir 기준 상대 경로·확장자 없음·`/` 구분 |
| `kg_watch_folder` | `dir` | 감시 번호 | 1초 간격으로 PNG 수정 시각을 비교해 바뀌면 `kg://folder-changed` 이벤트(`{dir}`) |
| `kg_unwatch_folder` | `id` | | |
| `kg_register_association` | | 메시지 | HKCU\Software\Classes에 `.kgbone` → `KGBone.Project` → `"<exe>" "%1"` 등록(관리자 권한 불필요) |
| `kg_launch_file` | | 경로 또는 null | 실행 인자 중 첫 번째 파일 경로(더블클릭·exe에 끌어 놓기) |

파일 대화상자는 `tauri-plugin-dialog`(JS API), 닫기 확인은 `getCurrentWindow().onCloseRequested`를 쓴다.
이미지 URL은 `convertFileSrc(path)`(asset 프로토콜, 범위 `**`).
