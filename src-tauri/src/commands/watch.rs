//! 이미지 폴더 감시. 외부 크레이트 없이 1초마다 PNG 목록·수정 시각·크기를 비교한다.
use super::images::scan_pngs;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Emitter, State};

/// 화면에 보내는 이벤트 이름.
const FOLDER_CHANGED: &str = "kg://folder-changed";
const INTERVAL: Duration = Duration::from_secs(1);

/// 실행 중인 감시들(감시 번호 → 멈춤 표시).
#[derive(Default)]
pub struct Watchers {
    next_id: AtomicU32,
    stops: Mutex<HashMap<u32, Arc<AtomicBool>>>,
}

#[derive(Serialize, Clone)]
struct FolderChanged {
    dir: String,
}

/// 폴더 상태 한 번 찍은 것: (경로, 수정 시각, 크기)
type Snapshot = Vec<(PathBuf, Option<SystemTime>, u64)>;

fn snapshot(dir: &Path) -> Snapshot {
    scan_pngs(dir)
        .unwrap_or_default()
        .into_iter()
        .map(|p| {
            let meta = std::fs::metadata(&p).ok();
            let modified = meta.as_ref().and_then(|m| m.modified().ok());
            let len = meta.map(|m| m.len()).unwrap_or(0);
            (p, modified, len)
        })
        .collect()
}

#[tauri::command]
pub fn kg_watch_folder(app: AppHandle, watchers: State<'_, Watchers>, dir: String) -> Result<u32, String> {
    if !Path::new(&dir).is_dir() {
        return Err(format!("감시할 폴더가 없습니다: {dir}"));
    }
    let id = watchers.next_id.fetch_add(1, Ordering::Relaxed) + 1;
    let stop = Arc::new(AtomicBool::new(false));
    watchers
        .stops
        .lock()
        .map_err(|_| "폴더 감시 상태를 읽지 못했습니다".to_string())?
        .insert(id, stop.clone());

    std::thread::Builder::new()
        .name(format!("kgbone-watch-{id}"))
        .spawn(move || {
            let root = PathBuf::from(&dir);
            let mut last = snapshot(&root);
            loop {
                std::thread::sleep(INTERVAL);
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let now = snapshot(&root);
                if now != last {
                    last = now;
                    let _ = app.emit(FOLDER_CHANGED, FolderChanged { dir: dir.clone() });
                }
            }
        })
        .map_err(|e| format!("폴더 감시를 시작하지 못했습니다: {e}"))?;
    Ok(id)
}

#[tauri::command]
pub fn kg_unwatch_folder(watchers: State<'_, Watchers>, id: u32) -> Result<(), String> {
    let mut stops = watchers.stops.lock().map_err(|_| "폴더 감시 상태를 읽지 못했습니다".to_string())?;
    if let Some(stop) = stops.remove(&id) {
        stop.store(true, Ordering::Relaxed);
    }
    Ok(())
}
