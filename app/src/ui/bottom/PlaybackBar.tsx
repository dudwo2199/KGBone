// 재생 막대: 처음·거꾸로·재생/정지·끝·반복·속도·시점·애니메이션 고르기·오니언 스킨.
import { clampSpeed } from '../../anim/playback';
import { setTime } from '../../commands/actions';
import { useServices } from '../../services';
import { useStore } from '../../state/store';
import { NumberField } from '../fields/NumberField';
import { formatNumber } from '../fields/numberInput';
import { Icon } from '../icons';

export function PlaybackBar() {
  const s = useServices();
  const playing = useStore(s.ui, (u) => u.playing);
  const reverse = useStore(s.ui, (u) => u.reverse);
  const loop = useStore(s.ui, (u) => u.loop);
  const speed = useStore(s.ui, (u) => u.speed);
  const onion = useStore(s.ui, (u) => u.onion);
  const playTime = useStore(s.ui, (u) => u.playTime);
  const docTime = useStore(s.doc, (d) => d.project.editor.time);
  const fps = useStore(s.doc, (d) => d.project.skeleton.fps || 30);
  const anims = useStore(s.doc, (d) => d.project.skeleton.animations);
  const active = useStore(s.doc, (d) => d.project.editor.activeAnimation);
  const time = playing ? playTime : docTime;
  const frame = Math.round(time * fps * 100) / 100;
  return (
    <div className="playbar" role="toolbar" aria-label="재생 막대">
      <select aria-label="애니메이션 고르기" data-testid="animation-select" value={active ?? ''} onChange={(e) => s.doc.run('animation.setActive', { name: e.target.value || null })}>
        <option value="">(애니메이션 없음)</option>
        {anims.map((a) => (
          <option key={a.name} value={a.name}>
            {a.name}
          </option>
        ))}
      </select>
      <button title="처음 (Ctrl+Home)" onClick={() => s.run('time.first')}>
        <Icon name="first" />
      </button>
      <button title="거꾸로 재생 (A)" className={playing && reverse ? 'on' : ''} onClick={() => s.run('play.reverse')}>
        <Icon name="reverse" />
      </button>
      <button title="재생 / 정지 (D)" data-testid="play-toggle" className={playing && !reverse ? 'on' : ''} onClick={() => s.run('play.toggle')}>
        <Icon name={playing ? 'stop' : 'play'} />
      </button>
      <button title="끝 (Ctrl+End)" onClick={() => s.run('time.last')}>
        <Icon name="last" />
      </button>
      <button title="반복" aria-pressed={loop} className={loop ? 'on' : ''} onClick={() => s.run('play.toggleLoop')}>
        <Icon name="loop" />
      </button>
      <label className="playbar-speed">
        속도
        <select aria-label="속도" value={String(speed)} onChange={(e) => s.ui.set({ speed: clampSpeed(Number(e.target.value)) })}>
          {[0.1, 0.25, 0.5, 1, 1.5, 2, 3, 4].map((v) => (
            <option key={v} value={v}>
              {v}배
            </option>
          ))}
        </select>
      </label>
      <div className="playbar-time">
        <NumberField label="프레임" value={frame} testId="time-frame" onCommit={(f) => setTime(s, f / fps)} />
        <span className="playbar-sec" data-testid="time-seconds">
          {formatNumber(time, 2)}초
        </span>
      </div>
      <button title="오니언 스킨 (O)" aria-pressed={onion} className={'text' + (onion ? ' on' : '')} onClick={() => s.run('view.toggleOnion')}>
        오니언
      </button>
    </div>
  );
}
