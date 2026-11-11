//! .kgbone 파일 연결 등록(현재 사용자, 관리자 권한 불필요). 레지스트리는 reg.exe로 쓴다(외부 크레이트 없음).
use std::process::Command;

const EXTENSION_KEY: &str = r"HKCU\Software\Classes\.kgbone";
const PROG_ID: &str = "KGBone.Project";
const PROG_KEY: &str = r"HKCU\Software\Classes\KGBone.Project";

/// 등록할 (키, 기본값) 목록.
pub(crate) fn association_entries(exe: &str) -> Vec<(String, String)> {
    vec![
        (EXTENSION_KEY.to_string(), PROG_ID.to_string()),
        (PROG_KEY.to_string(), "KGBone 프로젝트".to_string()),
        (format!(r"{PROG_KEY}\DefaultIcon"), format!("\"{exe}\",0")),
        (format!(r"{PROG_KEY}\shell\open\command"), format!("\"{exe}\" \"%1\"")),
    ]
}

#[tauri::command]
pub async fn kg_register_association() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("실행 파일 위치를 알 수 없습니다: {e}"))?;
    let exe = exe.to_string_lossy().to_string();
    for (key, value) in association_entries(&exe) {
        set_default_value(&key, &value)?;
    }
    Ok(format!(".kgbone 파일을 KGBone으로 열도록 등록했습니다.\n실행 파일: {exe}"))
}

/// reg add "<key>" /ve /d "<value>" /f
fn set_default_value(key: &str, value: &str) -> Result<(), String> {
    let mut cmd = Command::new("reg.exe");
    cmd.args(["add", key, "/ve", "/t", "REG_SZ", "/d", value, "/f"]);
    hide_console(&mut cmd);
    let out = cmd.output().map_err(|e| format!("reg.exe를 실행하지 못했습니다: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let detail = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(format!("파일 연결을 등록하지 못했습니다: {key} ({detail})"))
    }
}

#[cfg(windows)]
fn hide_console(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console(_cmd: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f_j09_association_entries() {
        let e = association_entries(r"C:\앱\KGBone.exe");
        assert_eq!(e[0], (r"HKCU\Software\Classes\.kgbone".into(), "KGBone.Project".into()));
        assert_eq!(e[1].1, "KGBone 프로젝트");
        assert_eq!(e[2], (r"HKCU\Software\Classes\KGBone.Project\DefaultIcon".into(), r#""C:\앱\KGBone.exe",0"#.into()));
        assert_eq!(
            e[3],
            (r"HKCU\Software\Classes\KGBone.Project\shell\open\command".into(), r#""C:\앱\KGBone.exe" "%1""#.into())
        );
    }
}
