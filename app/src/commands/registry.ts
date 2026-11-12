// 명령 등록표: interface/commands.json의 명령 id → 처리기. 메뉴·단축키·버튼이 모두 여기로 명령을 보낸다.
import spec from '../../../interface/commands.json';

export type Scope = 'core' | 'ui' | 'platform';

export interface CommandDef {
  id: string;
  label: string;
  menu: string;
  shortcut: string[];
  scope: Scope;
  args: Record<string, string>;
}

export const COMMAND_DEFS: readonly CommandDef[] = (spec as unknown as { commands: CommandDef[] }).commands;

const byId = new Map(COMMAND_DEFS.map((c) => [c.id, c]));

export function commandDef(id: string): CommandDef | undefined {
  return byId.get(id);
}

export function commandLabel(id: string): string {
  return byId.get(id)?.label ?? id;
}

export type Args = Record<string, unknown>;

/** 처리기. ctx는 실행할 때 받는다(등록표 자체는 화면 없이 만들 수 있다). */
export type Handler<C> = (ctx: C, args: Args) => void | Promise<void>;

export class CommandRegistry<C> {
  private handlers = new Map<string, Handler<C>>();

  /**
   * @param coreFallback 처리기를 따로 등록하지 않은 scope=core 명령을 실행하는 기본 처리기
   */
  constructor(private coreFallback: (ctx: C, id: string, args: Args) => void) {}

  register(id: string, h: Handler<C>): this {
    if (!byId.has(id)) throw new Error(`commands.json에 없는 명령: ${id}`);
    this.handlers.set(id, h);
    return this;
  }

  /** 명령을 처리할 수 있는가(scope=core는 기본 처리기가 있다) */
  has(id: string): boolean {
    if (this.handlers.has(id)) return true;
    return byId.get(id)?.scope === 'core';
  }

  /** 따로 등록된 처리기가 있는가 */
  hasOwn(id: string): boolean {
    return this.handlers.has(id);
  }

  async run(ctx: C, id: string, args: Args = {}): Promise<void> {
    const h = this.handlers.get(id);
    if (h) return await h(ctx, args);
    const def = byId.get(id);
    if (def?.scope === 'core') return this.coreFallback(ctx, id, args);
    throw new Error(`처리기가 없는 명령: ${id}`);
  }
}
