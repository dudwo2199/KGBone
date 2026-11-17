// 상태 표시줄: 선택, 커서 월드 좌표, 확대 비율, 경고·오류 메시지.
import { useServices } from '../../services';
import { useStore } from '../../state/store';
import { formatNumber } from '../fields/numberInput';

const KIND: Record<string, string> = {
  skeleton: '뼈대',
  bone: '본',
  slot: '슬롯',
  attachment: '첨부물',
  constraint: '제약',
  skin: '스킨',
  event: '이벤트',
  animation: '애니메이션',
  image: '이미지',
};

export function StatusBar() {
  const s = useServices();
  const sel = useStore(s.ui, (u) => u.selection);
  const cursor = useStore(s.ui, (u) => u.cursor);
  const zoom = useStore(s.ui, (u) => u.view.zoom);
  const status = useStore(s.ui, (u) => u.status);
  const messages = useStore(s.ui, (u) => u.messages);
  const playing = useStore(s.ui, (u) => u.playing);
  const pick = useStore(s.ui, (u) => u.pickParent);
  const agent = useStore(s.agent, (a) => a.status);
  const agentClients = useStore(s.agent, (a) => a.clients);
  const warnCount = messages.filter((m) => m.level !== 'info').length;
  const selText = sel.length === 0 ? '선택 없음' : sel.length === 1 ? `${KIND[sel[0].kind]}: ${sel[0].name}` : `${sel.length}개 선택`;
  return (
    <div className="statusbar" data-testid="statusbar">
      <span className="status-sel">{selText}</span>
      <span className="status-cursor">{cursor ? `X ${formatNumber(cursor.x, 1)}  Y ${formatNumber(cursor.y, 1)}` : ''}</span>
      <span className="status-zoom">{Math.round(zoom * 100)}%</span>
      {playing && <span className="status-playing">재생 중</span>}
      {pick && <span className="status-pick">부모 본 고르기…</span>}
      <span className={'status-msg ' + (status?.level ?? '')} data-testid="status-message" title={status?.text}>
        {status?.text ?? '준비됨'}
      </span>
      {agent !== 'off' && (
        <button
          className={'status-ai ' + agent}
          data-testid="status-ai"
          title={agentClients.length ? `붙은 AI: ${agentClients.join(', ')}` : 'MCP 연결'}
          onClick={() => s.run('ai.connect')}
        >
          {agent === 'connected' ? `MCP 연결됨${agentClients.length ? ` (${agentClients.length})` : ''}` : 'MCP 기다리는 중'}
        </button>
      )}
      <span className="status-warn" title="경고·오류 수">
        경고 {warnCount}
      </span>
    </div>
  );
}
