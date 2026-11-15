// J09(브라우저 판): 설치한 웹 앱이 더블클릭으로 받은 파일을 꺼낸다
import { describe, expect, it, vi } from 'vitest';
import { firstLaunchFile, type LaunchQueueLike } from './launchQueue';

type Params = { files?: readonly string[] };

function fakeQueue(deliver: (consumer: (p: Params) => void) => void): LaunchQueueLike<string> {
  return { setConsumer: deliver };
}

describe('f_j09 설치한 웹 앱의 실행 파일', () => {
  it('f_j09_launch_queue_returns_first_file', async () => {
    const q = fakeQueue((c) => c({ files: ['a.kgbone', 'b.kgbone'] }));
    expect(await firstLaunchFile(q)).toBe('a.kgbone');
  });

  it('f_j09_launch_queue_missing_is_null', async () => {
    expect(await firstLaunchFile<string>(undefined)).toBeNull();
  });

  it('f_j09_launch_queue_without_files_times_out_to_null', async () => {
    vi.useFakeTimers();
    try {
      // 파일 없이 열림: 소비자가 안 불리거나 빈 목록으로 불린다
      const silent = firstLaunchFile(fakeQueue(() => {}), 200);
      const empty = firstLaunchFile(fakeQueue((c) => c({ files: [] })), 200);
      await vi.advanceTimersByTimeAsync(200);
      expect(await silent).toBeNull();
      expect(await empty).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });

  it('f_j09_launch_queue_late_file_within_wait_is_returned', async () => {
    vi.useFakeTimers();
    try {
      const p = firstLaunchFile(fakeQueue((c) => void setTimeout(() => c({ files: ['late.kgbone'] }), 50)), 200);
      await vi.advanceTimersByTimeAsync(50);
      expect(await p).toBe('late.kgbone');
    } finally {
      vi.useRealTimers();
    }
  });
});
