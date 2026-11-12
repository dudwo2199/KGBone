// 아래쪽 영역(애니메이트 모드): 재생 막대 + 도프시트/그래프 탭.
import { useServices } from '../../services';
import { useStore } from '../../state/store';
import { Dopesheet } from './Dopesheet';
import { GraphEditor } from './GraphEditor';
import { PlaybackBar } from './PlaybackBar';

export function BottomPanel() {
  const s = useServices();
  const tab = useStore(s.ui, (u) => u.bottomTab);
  return (
    <div className="bottom-panel" data-panel="bottom">
      <div className="bottom-head">
        <PlaybackBar />
        <div className="tabs" role="tablist">
          <button role="tab" aria-selected={tab === 'dopesheet'} className={tab === 'dopesheet' ? 'on' : ''} onClick={() => s.run('view.showDopesheet')}>
            도프시트
          </button>
          <button role="tab" aria-selected={tab === 'graph'} className={tab === 'graph' ? 'on' : ''} onClick={() => s.run('view.showGraph')}>
            그래프
          </button>
        </div>
      </div>
      <div className="bottom-body">{tab === 'dopesheet' ? <Dopesheet /> : <GraphEditor />}</div>
    </div>
  );
}
