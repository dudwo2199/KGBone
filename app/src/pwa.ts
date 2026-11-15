// 설치형 웹 앱(PWA) 준비: 배포 빌드를 브라우저에서 열 때만 서비스 워커를 등록한다.
// 개발 서버(캐시가 방해됨)와 Tauri 앱(파일이 앱 안에 들어 있음)에서는 등록하지 않는다.

export function registerServiceWorker(): void {
  if (!import.meta.env.PROD) return;
  if (typeof navigator === 'undefined' || !('serviceWorker' in navigator)) return;
  if ('__TAURI_INTERNALS__' in window) return;
  // 등록에 실패해도(예: 보안 연결이 아님) 앱은 그대로 쓸 수 있다
  navigator.serviceWorker.register(`${import.meta.env.BASE_URL}sw.js`).catch(() => {});
}
