// 화면 시작점: 핵심(wasm) 불러오기 → 플랫폼 고르기 → 실행 인자 파일 열기 → App.
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { loadCore } from './core/wasm';
import { createPlatform } from './platform';
import { registerServiceWorker } from './pwa';
import { createServices, ServicesContext } from './services';
import { App } from './ui/App';
import { DialogService } from './ui/dialogs/service';
import './ui/styles.css';

async function start() {
  const root = createRoot(document.getElementById('root')!);
  let core;
  try {
    core = await loadCore(`${import.meta.env.BASE_URL}kgbone.wasm`);
    core.newDocument();
  } catch (e) {
    root.render(<div className="fatal">핵심 모듈(kgbone.wasm)을 불러오지 못했습니다: {e instanceof Error ? e.message : String(e)}</div>);
    return;
  }
  const dialogs = new DialogService();
  const platform = await createPlatform(dialogs);
  const s = createServices(core, platform, dialogs);
  platform.onCloseRequested(() => s.files.confirmDiscard());
  root.render(
    <StrictMode>
      <ServicesContext.Provider value={s}>
        <App />
      </ServicesContext.Provider>
    </StrictMode>,
  );
  s.doc.refresh();
  await s.images.open(s.files.imagesDir());
  try {
    const file = await platform.launchFile();
    if (file) await s.files.open(file, { skipConfirm: true });
  } catch (e) {
    s.ui.notify('error', `실행 인자 파일을 열지 못했습니다: ${e instanceof Error ? e.message : String(e)}`);
  }
  s.ui.set({ ready: true });
  // AI 연결: 자동 연결을 켜 두었으면 잇는다
  s.agent.autoStart();
  // 자동 테스트·디버그용
  (window as unknown as { __kgbone?: unknown }).__kgbone = s;
}

registerServiceWorker();
void start();
