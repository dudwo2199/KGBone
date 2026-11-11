//! KGBone 앱 본체. 디스크·창·레지스트리만 다루고, 뼈대 계산·파일 형식은 kgbone-core(화면에서는 WebAssembly)가 한다.
//! 명령 목록은 interface/tauri-commands.md와 1:1.
mod commands;

use commands::watch::Watchers;

/// 앱을 실행한다(main.rs에서 부름).
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Watchers::default())
        .invoke_handler(tauri::generate_handler![
            commands::fs::kg_read_text,
            commands::fs::kg_write_text,
            commands::fs::kg_read_binary,
            commands::fs::kg_write_binary,
            commands::images::kg_list_images,
            commands::watch::kg_watch_folder,
            commands::watch::kg_unwatch_folder,
            commands::assoc::kg_register_association,
            commands::launch::kg_launch_file,
        ])
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("KGBone을 시작하지 못했습니다");
}
