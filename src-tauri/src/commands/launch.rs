//! 실행 인자로 받은 파일(.kgbone 더블클릭, exe 아이콘에 끌어 놓기).
use std::path::Path;

#[tauri::command]
pub fn kg_launch_file() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    parse_launch_args(&args)
}

/// 실행 인자에서 첫 번째로 존재하는 파일 경로를 고른다.
/// args[0]은 실행 파일 자신이므로 건너뛰고, '-'로 시작하는 옵션도 건너뛴다.
/// 상대 경로는 현재 폴더 기준 절대 경로로 바꿔 돌려준다.
pub fn parse_launch_args(args: &[String]) -> Option<String> {
    args.iter()
        .skip(1)
        .map(|a| a.trim())
        .filter(|a| !a.is_empty() && !a.starts_with('-'))
        .find(|a| Path::new(a).is_file())
        .map(|a| match std::path::absolute(a) {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => a.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kgbone-launch-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, b"{}").unwrap();
        p
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn f_j09_no_args_gives_none() {
        assert_eq!(parse_launch_args(&[]), None);
        assert_eq!(parse_launch_args(&s(&["KGBone.exe"])), None);
    }

    #[test]
    fn f_j09_exe_itself_is_skipped() {
        // args[0]이 존재하는 파일이어도 고르지 않는다
        let exe = temp_file("self.exe");
        assert_eq!(parse_launch_args(&s(&[exe.to_str().unwrap()])), None);
    }

    #[test]
    fn f_j09_first_existing_file_wins() {
        let a = temp_file("첫번째.kgbone");
        let b = temp_file("second.kgbone");
        let args = s(&[
            "KGBone.exe",
            "--flag",
            "없는파일.kgbone",
            a.to_str().unwrap(),
            b.to_str().unwrap(),
        ]);
        let got = parse_launch_args(&args).expect("파일을 찾아야 한다");
        assert_eq!(PathBuf::from(got), std::path::absolute(&a).unwrap());
    }

    #[test]
    fn f_j09_folder_is_not_a_file() {
        let dir = std::env::temp_dir();
        assert_eq!(parse_launch_args(&s(&["KGBone.exe", dir.to_str().unwrap()])), None);
    }

    #[test]
    fn f_j09_quoted_path_with_spaces() {
        // 탐색기는 "%1"로 따옴표를 붙여 넘기지만, 인자로 나뉠 때 따옴표는 빠지고 공백은 남는다
        let a = temp_file("공백 있는 이름.kgbone");
        let got = parse_launch_args(&s(&["KGBone.exe", a.to_str().unwrap()])).unwrap();
        assert!(got.ends_with("공백 있는 이름.kgbone"));
    }
}
