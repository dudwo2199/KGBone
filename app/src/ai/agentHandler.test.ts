// AI 연결로 온 화면 명령: 대화상자를 여는 명령은 기다리지 않고 바로 닫고 실패로 알린다.
import { describe, expect, it } from 'vitest';
import type { Services } from '../services';
import { DialogService } from '../ui/dialogs/service';
import { runUiCommand } from './agentHandler';

function fake(run: (s: Services, id: string) => Promise<unknown>) {
  const dialogs = new DialogService();
  const s = { dialogs, commands: { run: (ctx: Services, id: string) => run(ctx, id) } } as unknown as Services;
  return { s, dialogs };
}

describe('AI 연결 화면 명령', () => {
  it('f_n01_ui_command_that_opens_dialog_is_cancelled_and_fails', async () => {
    let answer: unknown = 'not set';
    const { s, dialogs } = fake(async (ctx) => {
      answer = await ctx.dialogs.pickImage(['a', 'b']);
    });
    await expect(runUiCommand(s, 'attachment.attachImage', {})).rejects.toThrow(/대화상자\(이미지 고르기\)/);
    // 명령은 '취소'를 받고 끝났고, 대화상자는 남아 있지 않다
    expect(answer).toBeNull();
    expect(dialogs.get().stack).toHaveLength(0);
  });

  it('f_n01_ui_command_without_dialog_succeeds_and_leaves_user_dialogs', async () => {
    const { s, dialogs } = fake(async () => undefined);
    // 사용자가 미리 열어 둔 대화상자는 건드리지 않는다
    void dialogs.shortcuts();
    await runUiCommand(s, 'view.fit', {});
    expect(dialogs.get().stack).toHaveLength(1);
  });

  it('f_n01_cancel_values_match_dialog_kind', async () => {
    const dialogs = new DialogService();
    const save = dialogs.confirmSave();
    const ok = dialogs.confirm('t', 'x', '예', '아니요');
    const msg = dialogs.message('t', 'x');
    for (const d of [...dialogs.get().stack]) dialogs.cancel(d);
    expect(await save).toBe('cancel');
    expect(await ok).toBe(false);
    expect(await msg).toBeUndefined();
  });
});
