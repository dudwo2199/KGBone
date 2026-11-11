//! 이미지 폴더의 PNG 목록(하위 폴더 포함).
use super::io_error;
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};

/// 화면에 돌려주는 이미지 한 개. 화면은 path로 asset URL을 만든다.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct ImageFile {
    /// dir 기준 상대 경로, 확장자 없음, '/' 구분(첨부물 path와 같은 값)
    pub name: String,
    /// 전체 경로
    pub path: String,
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub async fn kg_list_images(dir: String) -> Result<Vec<ImageFile>, String> {
    let root = Path::new(&dir);
    if !root.is_dir() {
        return Err(format!("이미지 폴더가 없습니다: {dir}"));
    }
    let mut out = Vec::new();
    for file in scan_pngs(root).map_err(|e| io_error("이미지 폴더를 읽지", &dir, e))? {
        // 크기는 파일 앞부분(IHDR)만 읽어 알아낸다. PNG가 아니면 건너뛴다.
        let Some((width, height)) = read_head(&file).and_then(|h| kgbone_core::export::png_size(&h)) else {
            continue;
        };
        out.push(ImageFile {
            name: image_name(root, &file),
            path: file.to_string_lossy().to_string(),
            width,
            height,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// dir 아래(하위 폴더 포함) .png 파일 경로. 확장자는 대소문자를 가리지 않는다.
/// 바로가기(심볼릭 링크) 폴더는 따라가지 않는다(순환 방지).
pub(crate) fn scan_pngs(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut first = true;
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            // 최상위 폴더를 못 읽으면 오류, 하위 폴더는 건너뛴다
            Err(e) if first => return Err(e),
            Err(_) => continue,
        };
        first = false;
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else { continue };
            let path = entry.path();
            if kind.is_dir() {
                stack.push(path);
            } else if kind.is_file() && is_png(&path) {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

fn is_png(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("png"))
}

/// 파일 앞 64바이트만 읽는다.
fn read_head(path: &Path) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mut head = Vec::with_capacity(64);
    file.take(64).read_to_end(&mut head).ok()?;
    Some(head)
}

/// root 기준 상대 경로에서 확장자를 떼고 '/'로 잇는다. 예: root\arms\left.png → "arms/left"
pub(crate) fn image_name(root: &Path, file: &Path) -> String {
    let rel = file.strip_prefix(root).unwrap_or(file).with_extension("");
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f_j08_image_name_relative_without_extension() {
        let root = Path::new("C:\\proj\\images");
        let file = root.join("arms").join("left.png");
        assert_eq!(image_name(root, &file), "arms/left");
        assert_eq!(image_name(root, &root.join("head.PNG")), "head");
        // 점이 들어간 이름은 마지막 확장자만 뗀다
        assert_eq!(image_name(root, &root.join("a.b.png")), "a.b");
    }

    #[test]
    fn f_j08_scan_pngs_recursive_png_only() {
        let base = std::env::temp_dir().join(format!("kgbone-scan-test-{}", std::process::id()));
        let sub = base.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(base.join("a.png"), b"x").unwrap();
        std::fs::write(base.join("b.txt"), b"x").unwrap();
        std::fs::write(sub.join("c.PNG"), b"x").unwrap();
        let names: Vec<String> = scan_pngs(&base).unwrap().iter().map(|p| image_name(&base, p)).collect();
        assert_eq!(names, vec!["a".to_string(), "sub/c".to_string()]);
        let _ = std::fs::remove_dir_all(&base);
    }
}
