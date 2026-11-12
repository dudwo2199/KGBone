// 앱 뼈대: 메뉴 막대 / 뷰포트+도구 모음 / 트리+속성 / 아래쪽(애니메이트 모드) / 상태 표시줄, 단축키 처리.
import { useEffect } from 'react';
import { CompositionTracker, findCommand, isTextField, shouldDispatch } from '../commands/shortcuts';
import { useServices } from '../services';
import { useStore } from '../state/store';
import { BottomPanel } from './bottom/BottomPanel';
import { DialogHost } from './dialogs/DialogHost';
import { MenuBar } from './panels/MenuBar';
import { StatusBar } from './panels/StatusBar';
import { ModeSwitch, Toolbar } from './panels/Toolbar';
import { Viewport } from './panels/Viewport';
import { Properties } from './props/Properties';
import { Tree } from './tree/Tree';

function useShortcuts() {
  const s = useServices();
  useEffect(() => {
    const tracker = new CompositionTracker();
    const detach = tracker.attach(document);
    const onKey = (e: KeyboardEvent) => {
      if (s.dialogs.isOpen()) return;
      const ctx = { inTextField: isTextField(document.activeElement), composing: tracker.composing };
      if (!shouldDispatch(e, ctx)) return;
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        (document.querySelector('[data-testid="tree-search"]') as HTMLInputElement | null)?.focus();
        return;
      }
      const id = findCommand(e);
      if (!id) return;
      // 입력칸 안의 Ctrl+C/V/X/A 같은 기본 편집은 그대로 둔다(명령표에 없으므로 여기 오지 않음)
      e.preventDefault();
      s.run(id);
    };
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
      detach();
    };
  }, [s]);
}

function useWindowTitle() {
  const s = useServices();
  const dirty = useStore(s.doc, (d) => d.docState.dirty);
  const name = useStore(s.doc, (d) => d.filePath ?? d.untitledName);
  useEffect(() => {
    s.platform.setTitle(`KGBone - ${s.doc.displayName()}${dirty ? '*' : ''}`);
  }, [s, dirty, name]);
}

export function App() {
  const s = useServices();
  const mode = useStore(s.doc, (d) => d.project.editor.mode);
  const ready = useStore(s.ui, (u) => u.ready);
  useShortcuts();
  useWindowTitle();
  return (
    <div className={'app' + (mode === 'animate' ? ' animate' : '')} data-ready={ready ? 'true' : 'false'} data-mode={mode}>
      <MenuBar />
      <div className="main">
        <div className="center">
          <div className="viewport-wrap">
            <Viewport />
            <div className="viewport-top">
              <ModeSwitch />
            </div>
          </div>
          <Toolbar />
        </div>
        <div className="side">
          <Tree />
          <Properties />
        </div>
      </div>
      {mode === 'animate' && <BottomPanel />}
      <StatusBar />
      <DialogHost />
    </div>
  );
}
