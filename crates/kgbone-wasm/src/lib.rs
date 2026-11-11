//! KGBone 핵심을 화면에서 쓰기 위한 WebAssembly 연결층.
//!
//! wasm-bindgen 없이 C ABI 함수만 내보낸다(이 PC는 빌드 스크립트 실행이 막혀 wasm-bindgen을 빌드할 수 없다).
//! 화면 쪽 짝은 `app/src/core/wasm.ts`. 문자열은 UTF-8 바이트로 선형 메모리를 통해 주고받는다.
//!
//! 내보내는 함수:
//! - `kg_alloc(len) -> ptr`, `kg_free(ptr, len)`: 메모리 빌리기·돌려주기
//! - `kg_call(ptr, len) -> u32`: JSON 요청 처리(`kgbone_core::api::Session::handle_text`). 결과는 `kg_result_ptr/len`으로 읽는다
//! - `kg_set_image(path_ptr, path_len, w, h, alpha_ptr, alpha_len)`: 이미지 크기·알파 등록
//! - `kg_add_png(path_ptr, path_len, png_ptr, png_len)`: 내보내기용 PNG 원본 등록
use kgbone_core::api::Session;
use std::cell::RefCell;

thread_local! {
    static SESSION: RefCell<Session> = RefCell::new(Session::new());
    static RESULT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// # Safety
/// 화면 쪽은 돌려받은 주소에 `len` 바이트까지만 쓴다.
#[no_mangle]
pub extern "C" fn kg_alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// # Safety
/// `ptr`은 `kg_alloc(len)`이 돌려준 주소여야 한다.
#[no_mangle]
pub unsafe extern "C" fn kg_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
    }
}

unsafe fn bytes<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if ptr.is_null() || len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(ptr, len)
    }
}

/// JSON 요청 처리. 응답 길이를 돌려준다(내용은 `kg_result_ptr`).
///
/// # Safety
/// `ptr..ptr+len`은 유효한 UTF-8 바이트여야 한다.
#[no_mangle]
pub unsafe extern "C" fn kg_call(ptr: *const u8, len: usize) -> usize {
    let req = String::from_utf8_lossy(bytes(ptr, len)).into_owned();
    let resp = SESSION.with(|s| s.borrow_mut().handle_text(&req));
    RESULT.with(|r| {
        let mut r = r.borrow_mut();
        *r = resp.into_bytes();
        r.len()
    })
}

/// 마지막 응답의 주소.
#[no_mangle]
pub extern "C" fn kg_result_ptr() -> *const u8 {
    RESULT.with(|r| r.borrow().as_ptr())
}

/// # Safety
/// 포인터·길이 쌍은 유효해야 한다.
#[no_mangle]
pub unsafe extern "C" fn kg_set_image(path_ptr: *const u8, path_len: usize, width: u32, height: u32, alpha_ptr: *const u8, alpha_len: usize) {
    let path = String::from_utf8_lossy(bytes(path_ptr, path_len)).into_owned();
    let alpha = bytes(alpha_ptr, alpha_len);
    SESSION.with(|s| s.borrow_mut().set_image(&path, width, height, alpha));
}

/// # Safety
/// 포인터·길이 쌍은 유효해야 한다.
#[no_mangle]
pub unsafe extern "C" fn kg_add_png(path_ptr: *const u8, path_len: usize, png_ptr: *const u8, png_len: usize) {
    let path = String::from_utf8_lossy(bytes(path_ptr, path_len)).into_owned();
    let png = bytes(png_ptr, png_len);
    SESSION.with(|s| s.borrow_mut().add_png(&path, png));
}
