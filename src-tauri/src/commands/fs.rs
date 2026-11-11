//! 파일 읽기·쓰기 명령. 무거운 입출력이 화면을 멈추지 않도록 async 명령(별도 스레드)으로 둔다.
use super::io_error;
use std::path::Path;
use tauri::ipc::Response;

/// UTF-8 텍스트를 읽는다(내용은 바꾸지 않는다: 무편집 저장 = 바이트 동일).
#[tauri::command]
pub async fn kg_read_text(path: String) -> Result<String, String> {
    let bytes = std::fs::read(&path).map_err(|e| io_error("파일을 읽지", &path, e))?;
    String::from_utf8(bytes).map_err(|_| format!("UTF-8 텍스트 파일이 아닙니다: {path}"))
}

/// 텍스트를 쓴다. 부모 폴더가 없으면 만든다.
#[tauri::command]
pub async fn kg_write_text(path: String, text: String) -> Result<(), String> {
    write_with_parents(&path, text.as_bytes())
}

/// 바이너리 파일을 raw 응답으로 돌려준다(화면에서는 ArrayBuffer).
#[tauri::command]
pub async fn kg_read_binary(path: String) -> Result<Response, String> {
    let bytes = std::fs::read(&path).map_err(|e| io_error("파일을 읽지", &path, e))?;
    Ok(Response::new(bytes))
}

/// base64 문자열을 바이트로 풀어 쓴다. 부모 폴더가 없으면 만든다.
#[tauri::command]
pub async fn kg_write_binary(path: String, base64: String) -> Result<(), String> {
    let bytes = kgbone_core::export::base64::decode(&base64)
        .map_err(|e| format!("base64 데이터를 풀지 못했습니다: {e}"))?;
    write_with_parents(&path, &bytes)
}

fn write_with_parents(path: &str, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| io_error("폴더를 만들지", &parent.to_string_lossy(), e))?;
        }
    }
    std::fs::write(path, bytes).map_err(|e| io_error("파일을 쓰지", path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f_j01_write_creates_parent_folders() {
        let base = std::env::temp_dir().join(format!("kgbone-fs-test-{}", std::process::id()));
        let file = base.join("a").join("b").join("c.txt");
        let path = file.to_string_lossy().to_string();
        write_with_parents(&path, "가나다".as_bytes()).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "가나다");
        let _ = std::fs::remove_dir_all(&base);
    }
}
