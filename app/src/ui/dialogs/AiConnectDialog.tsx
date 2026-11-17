// 도구 → AI 연결: 이 창을 AI 프로그램(Claude Code, Cursor, Codex …)이 띄운 KGBone MCP 서버에 잇는다.
import { useState } from 'react';
import type { AgentLinkState } from '../../ai/agentLink';
import { useServices } from '../../services';
import { useStore } from '../../state/store';
import { Modal } from './DialogHost';

const STATUS_TEXT: Record<AgentLinkState['status'], string> = {
  off: '꺼짐',
  connecting: '연결하는 중…',
  connected: '연결됨',
  waiting: 'AI 프로그램을 기다리는 중(MCP 서버가 켜지면 저절로 붙습니다)',
  busy: '다른 KGBone 창이 연결돼 있어 기다리는 중(그 창이 끊기면 이어받습니다. 이 창으로 가져오려면 [연결 끊기] 뒤 [연결])',
};

function ago(at: number): string {
  const s = Math.max(0, Math.round((Date.now() - at) / 1000));
  return s < 60 ? `${s}초 전` : `${Math.round(s / 60)}분 전`;
}

export function AiConnectDialog({ done }: { done(): void }) {
  const s = useServices();
  const st = useStore(s.agent, (a) => a);
  const [port, setPort] = useState(String(st.port));
  const on = st.status !== 'off';
  return (
    <Modal
      title="AI 연결"
      onCancel={done}
      buttons={
        <>
          {on ? (
            <button onClick={() => s.agent.disconnect()}>연결 끊기</button>
          ) : (
            <button className="primary" autoFocus onClick={() => s.agent.connect(Number(port))}>
              연결
            </button>
          )}
          <button onClick={done}>닫기</button>
        </>
      }
    >
      <div className="ai-connect" data-testid="ai-connect">
        <div className="ai-row">
          <span className="ai-label">상태</span>
          <span className={'ai-status ' + st.status} data-testid="ai-status">
            {STATUS_TEXT[st.status]}
            {st.status === 'connected' && ` (127.0.0.1:${st.port})`}
          </span>
        </div>
        {st.error && <div className="ai-error">{st.error}</div>}
        <div className="ai-row">
          <span className="ai-label">붙은 AI</span>
          <span data-testid="ai-clients">{st.clients.length ? st.clients.join(', ') : '없음'}</span>
        </div>
        <div className="ai-row">
          <span className="ai-label">최근 작업</span>
          <span data-testid="ai-last">{st.last ? `${st.last.client}: ${st.last.what}${st.last.ok ? '' : ' (실패)'} · ${ago(st.last.at)}` : '없음'}</span>
        </div>
        <label className="ai-row">
          <span className="ai-label">포트</span>
          <input
            aria-label="포트"
            value={port}
            disabled={on}
            inputMode="numeric"
            onChange={(e) => setPort(e.target.value.replace(/[^0-9]/g, ''))}
            style={{ width: '7em' }}
          />
        </label>
        <label className="ai-row">
          <input type="checkbox" checked={st.auto} onChange={(e) => s.agent.setAuto(e.target.checked)} />
          <span>창을 열 때 자동으로 연결</span>
        </label>
        <div className="props-subtitle">쓰는 법</div>
        <div className="pre ai-help">
          {'1. AI 프로그램에 KGBone MCP 서버를 등록한다(한 번만):\n' +
            '   명령 node, 인자 <KGBone 폴더>/tools/mcp/server.mjs\n' +
            '   Claude Code·Cursor·Codex·VS Code별 설정은 사용 설명서 "AI 연결" 장.\n' +
            '2. 여기서 [연결]을 누른다. AI가 고친 것은 화면에 바로 보이고 Ctrl+Z로 되돌린다.\n' +
            '여러 AI가 함께 붙으면 받은 차례대로 하나씩 실행한다.'}
        </div>
      </div>
    </Modal>
  );
}
