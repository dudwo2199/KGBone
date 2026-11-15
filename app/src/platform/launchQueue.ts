// 설치한 웹 앱(PWA)의 파일 연결: 탐색기에서 .kgbone을 더블클릭하면 브라우저가 launchQueue로 파일 핸들을 넘겨준다.
// 매니페스트(app/public/manifest.webmanifest)의 file_handlers가 연결을 선언한다.

/** window.launchQueue의 최소 모양(테스트에서 가짜로 바꿀 수 있게) */
export interface LaunchQueueLike<F> {
  setConsumer(consumer: (params: { files?: readonly F[] }) => void): void;
}

/**
 * 실행할 때 받은 첫 파일. 받은 파일이 없으면 null.
 * 브라우저는 파일 없이 열렸을 때 아무 것도 부르지 않을 수 있으므로 waitMs만 기다리고 포기한다.
 */
export function firstLaunchFile<F>(queue: LaunchQueueLike<F> | undefined, waitMs = 200): Promise<F | null> {
  if (!queue) return Promise.resolve(null);
  return new Promise((resolve) => {
    let done = false;
    const finish = (f: F | null) => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      resolve(f);
    };
    const timer = setTimeout(() => finish(null), waitMs);
    queue.setConsumer((params) => {
      const f = params.files?.[0];
      if (f) finish(f);
    });
  });
}
