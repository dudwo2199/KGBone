// 모드 전환(셋업|애니메이트)과 메인 도구 모음(뷰포트 아래). 버튼은 모두 명령 id를 부른다.
import { commandLabel } from '../../commands/registry';
import { shortcutText } from '../../commands/shortcuts';
import { useServices } from '../../services';
import { useStore } from '../../state/store';
import type { Axis, ToolId } from '../../state/ui';
import { Icon } from '../icons';

export function ModeSwitch() {
  const s = useServices();
  const mode = useStore(s.doc, (d) => d.project.editor.mode);
  const set = (m: 'setup' | 'animate') => {
    if (m === mode) return;
    s.run('mode.toggle');
  };
  return (
    <div className="mode-switch" role="group" aria-label="모드">
      <button className={mode === 'setup' ? 'on' : ''} data-testid="mode-setup" title="셋업 모드 (Tab)" onClick={() => set('setup')}>
        셋업
      </button>
      <button className={mode === 'animate' ? 'on animate' : ''} data-testid="mode-animate" title="애니메이트 모드 (Tab)" onClick={() => set('animate')}>
        애니메이트
      </button>
    </div>
  );
}

function ToolButton({ tool }: { tool: ToolId }) {
  const s = useServices();
  const active = useStore(s.ui, (u) => u.tool === tool);
  const id = `tool.${tool}`;
  const sc = shortcutText(id);
  return (
    <button className={'tool' + (active ? ' on' : '')} title={`${commandLabel(id)}${sc ? ` (${sc})` : ''}`} aria-pressed={active} onClick={() => s.run(id)}>
      <Icon name={tool} />
      <span>{commandLabel(id)}</span>
    </button>
  );
}

function AxisButton({ axis }: { axis: Axis }) {
  const s = useServices();
  const active = useStore(s.ui, (u) => u.axis === axis);
  const id = `axis.${axis}`;
  return (
    <button className={'tool text' + (active ? ' on' : '')} aria-pressed={active} onClick={() => s.run(id)}>
      {commandLabel(id)}
    </button>
  );
}

export function Toolbar() {
  const s = useServices();
  const compensate = useStore(s.ui, (u) => u.compensate);
  return (
    <div className="toolbar" role="toolbar" aria-label="메인 도구 모음">
      <div className="tool-group">
        <ToolButton tool="pose" />
        <ToolButton tool="weights" />
        <ToolButton tool="create" />
      </div>
      <div className="tool-group">
        <ToolButton tool="rotate" />
        <ToolButton tool="translate" />
        <ToolButton tool="scale" />
        <ToolButton tool="shear" />
      </div>
      <div className="tool-group">
        <AxisButton axis="local" />
        <AxisButton axis="parent" />
        <AxisButton axis="world" />
      </div>
      <div className="tool-group">
        <label className="tool text check">
          <input type="checkbox" checked={compensate} onChange={() => s.run('option.compensate')} />
          {commandLabel('option.compensate')}
        </label>
      </div>
    </div>
  );
}
