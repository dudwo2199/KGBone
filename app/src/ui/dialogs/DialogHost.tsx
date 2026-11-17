// 앱 안 대화상자 그리기(맨 위 하나). 모두 한국어, Playwright로 조작할 수 있게 역할·이름을 붙인다.
import { useState, type ReactNode } from 'react';
import { COMMAND_DEFS } from '../../commands/registry';
import type { PngSequenceChoice, SpineExportChoice } from '../../files/dialogs';
import { useServices } from '../../services';
import { useStore } from '../../state/store';
import { TextField } from '../fields/TextField';
import { AiConnectDialog } from './AiConnectDialog';
import type { DialogSpec, OpenDialog } from './service';

export function Modal({ title, children, buttons, onCancel }: { title: string; children: ReactNode; buttons: ReactNode; onCancel(): void }) {
  return (
    <div className="modal-back" onMouseDown={(e) => e.target === e.currentTarget && onCancel()}>
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-label={title}
        onKeyDown={(e) => {
          if (e.key === 'Escape' && !e.nativeEvent.isComposing) {
            e.stopPropagation();
            onCancel();
          }
          e.stopPropagation();
        }}
      >
        <div className="modal-title">{title}</div>
        <div className="modal-body">{children}</div>
        <div className="modal-buttons">{buttons}</div>
      </div>
    </div>
  );
}

function FileDialog({ spec, done }: { spec: Extract<DialogSpec, { kind: 'file' }>; done(v: string | null): void }) {
  const [path, setPath] = useState(spec.defaultPath);
  const okLabel = spec.mode === 'open' ? '열기' : spec.mode === 'save' ? '저장' : '고르기';
  const ok = () => path.trim() && done(path.trim());
  return (
    <Modal
      title={spec.title}
      onCancel={() => done(null)}
      buttons={
        <>
          <button className="primary" disabled={!path.trim()} onClick={ok}>
            {okLabel}
          </button>
          <button onClick={() => done(null)}>취소</button>
        </>
      }
    >
      <div className="file-list" role="listbox" aria-label={spec.mode === 'folder' ? '폴더 목록' : '파일 목록'}>
        {spec.files.length === 0 && <div className="props-note">(파일 없음)</div>}
        {spec.files.map((f) => (
          <div
            key={f}
            role="option"
            aria-selected={f === path}
            className={'file-item' + (f === path ? ' selected' : '')}
            onClick={() => setPath(f)}
            onDoubleClick={() => done(f)}
          >
            {f}
          </div>
        ))}
      </div>
      <label className="field wide">
        <span className="field-label">{spec.mode === 'folder' ? '폴더 경로' : '파일 경로'}</span>
        <input
          type="text"
          aria-label={spec.mode === 'folder' ? '폴더 경로' : '파일 경로'}
          value={path}
          autoFocus
          onChange={(e) => setPath(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.nativeEvent.isComposing) ok();
          }}
        />
      </label>
    </Modal>
  );
}

function PromptDialog({ spec, done }: { spec: Extract<DialogSpec, { kind: 'prompt' }>; done(v: string | null): void }) {
  const [v, setV] = useState(spec.initial);
  return (
    <Modal
      title={spec.title}
      onCancel={() => done(null)}
      buttons={
        <>
          <button className="primary" onClick={() => done(v)}>
            확인
          </button>
          <button onClick={() => done(null)}>취소</button>
        </>
      }
    >
      <label className="field wide">
        <span className="field-label">{spec.label}</span>
        <TextField aria-label={spec.label} value={v} autoSelect onLive={setV} onCommit={(x) => done(x)} onCancel={() => done(null)} />
      </label>
    </Modal>
  );
}

function ExportDialog({ spec, done }: { spec: Extract<DialogSpec, { kind: 'exportSpine' }>; done(v: SpineExportChoice | null): void }) {
  const [c, setC] = useState(spec.defaults);
  return (
    <Modal
      title="내보내기: Spine JSON 호환 + 아틀라스"
      onCancel={() => done(null)}
      buttons={
        <>
          <button className="primary" onClick={() => done(c)}>
            내보내기
          </button>
          <button onClick={() => done(null)}>취소</button>
        </>
      }
    >
      <label className="field wide">
        <span className="field-label">출력 폴더</span>
        <input type="text" aria-label="출력 폴더" value={c.dir} onChange={(e) => setC({ ...c, dir: e.target.value })} />
      </label>
      <label className="field">
        <span className="field-label">버전</span>
        <select aria-label="버전" value={c.version} onChange={(e) => setC({ ...c, version: e.target.value as '4.3' | '4.2' })}>
          <option value="4.3">4.3</option>
          <option value="4.2">4.2</option>
        </select>
      </label>
      <label className="field check">
        <input type="checkbox" checked={c.pretty} onChange={(e) => setC({ ...c, pretty: e.target.checked })} />
        <span>보기 좋게 출력</span>
      </label>
      <label className="field">
        <span className="field-label">아틀라스 최대 크기</span>
        <input type="number" aria-label="아틀라스 최대 크기" value={c.maxPageSize} onChange={(e) => setC({ ...c, maxPageSize: Number(e.target.value) || 2048 })} />
      </label>
      <label className="field">
        <span className="field-label">여백</span>
        <input type="number" aria-label="여백" value={c.padding} onChange={(e) => setC({ ...c, padding: Number(e.target.value) || 0 })} />
      </label>
    </Modal>
  );
}

function PngDialog({ spec, done }: { spec: Extract<DialogSpec, { kind: 'pngSequence' }>; done(v: PngSequenceChoice | null): void }) {
  const [c, setC] = useState(spec.defaults);
  const num = (label: string, key: 'fps' | 'scale' | 'width' | 'height') => (
    <label className="field">
      <span className="field-label">{label}</span>
      <input type="number" aria-label={label} value={c[key]} step={key === 'scale' ? 0.1 : 1} onChange={(e) => setC({ ...c, [key]: Number(e.target.value) || 0 })} />
    </label>
  );
  return (
    <Modal
      title="내보내기: PNG 시퀀스"
      onCancel={() => done(null)}
      buttons={
        <>
          <button className="primary" onClick={() => done(c)}>
            내보내기
          </button>
          <button onClick={() => done(null)}>취소</button>
        </>
      }
    >
      <label className="field wide">
        <span className="field-label">출력 폴더</span>
        <input type="text" aria-label="출력 폴더" value={c.dir} onChange={(e) => setC({ ...c, dir: e.target.value })} />
      </label>
      <label className="field">
        <span className="field-label">애니메이션</span>
        <select aria-label="애니메이션" value={c.animation ?? ''} onChange={(e) => setC({ ...c, animation: e.target.value || null })}>
          <option value="">(셋업 포즈)</option>
          {spec.animations.map((a) => (
            <option key={a} value={a}>
              {a}
            </option>
          ))}
        </select>
      </label>
      {num('fps', 'fps')}
      {num('배율', 'scale')}
      {num('폭(0=자동)', 'width')}
      {num('높이(0=자동)', 'height')}
      <label className="field check">
        <input type="checkbox" checked={c.transparent} onChange={(e) => setC({ ...c, transparent: e.target.checked })} />
        <span>배경 투명</span>
      </label>
    </Modal>
  );
}

function PickImageDialog({ spec, done }: { spec: Extract<DialogSpec, { kind: 'pickImage' }>; done(v: string | null): void }) {
  const s = useServices();
  const [sel, setSel] = useState<string | null>(spec.names[0] ?? null);
  return (
    <Modal
      title="이미지 고르기"
      onCancel={() => done(null)}
      buttons={
        <>
          <button className="primary" disabled={!sel} onClick={() => done(sel)}>
            붙이기
          </button>
          <button onClick={() => done(null)}>취소</button>
        </>
      }
    >
      {spec.names.length === 0 && <div className="props-note">이미지 폴더에 PNG가 없습니다 (파일 › 이미지 폴더 지정)</div>}
      <div className="image-grid" role="listbox" aria-label="이미지 목록">
        {spec.names.map((n) => (
          <div key={n} role="option" aria-label={n} aria-selected={sel === n} className={'image-cell' + (sel === n ? ' selected' : '')} onClick={() => setSel(n)} onDoubleClick={() => done(n)}>
            <img src={s.images.byName(n)?.url} alt="" />
            <span>{n}</span>
          </div>
        ))}
      </div>
    </Modal>
  );
}

function ShortcutsDialog({ done }: { done(): void }) {
  const list = COMMAND_DEFS.filter((c) => c.shortcut.length);
  return (
    <Modal title="단축키 목록" onCancel={done} buttons={<button onClick={done}>닫기</button>}>
      <table className="shortcut-table">
        <tbody>
          {list.map((c) => (
            <tr key={c.id}>
              <td>{c.label}</td>
              <td>{c.shortcut.map((x) => (x === 'Escape' ? 'Esc' : x)).join(', ')}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </Modal>
  );
}

function DialogView({ d }: { d: OpenDialog }) {
  const spec = d.spec;
  switch (spec.kind) {
    case 'confirmSave':
      return (
        <Modal
          title="저장 확인"
          onCancel={() => d.resolve('cancel')}
          buttons={
            <>
              <button className="primary" autoFocus onClick={() => d.resolve('save')}>
                저장
              </button>
              <button onClick={() => d.resolve('discard')}>저장 안 함</button>
              <button onClick={() => d.resolve('cancel')}>취소</button>
            </>
          }
        >
          저장하지 않은 변경이 있습니다. 저장할까요?
        </Modal>
      );
    case 'confirm':
      return (
        <Modal
          title={spec.title}
          onCancel={() => d.resolve(false)}
          buttons={
            <>
              <button className="primary" autoFocus onClick={() => d.resolve(true)}>
                {spec.ok}
              </button>
              <button onClick={() => d.resolve(false)}>{spec.cancel}</button>
            </>
          }
        >
          <div className="pre">{spec.text}</div>
        </Modal>
      );
    case 'message':
      return (
        <Modal title={spec.title} onCancel={() => d.resolve(undefined)} buttons={<button autoFocus onClick={() => d.resolve(undefined)}>확인</button>}>
          <div className="pre">{spec.text}</div>
        </Modal>
      );
    case 'report':
      return (
        <Modal title={spec.title} onCancel={() => d.resolve(undefined)} buttons={<button autoFocus onClick={() => d.resolve(undefined)}>확인</button>}>
          {spec.lines.length > 0 && (
            <>
              <div className="props-subtitle">파일</div>
              <ul className="report-list">
                {spec.lines.map((l) => (
                  <li key={l}>{l}</li>
                ))}
              </ul>
            </>
          )}
          <div className="props-subtitle">경고 {spec.warnings.length}개</div>
          <ul className="report-list warn">
            {spec.warnings.map((l, i) => (
              <li key={i}>{l}</li>
            ))}
          </ul>
        </Modal>
      );
    case 'prompt':
      return <PromptDialog spec={spec} done={(v) => d.resolve(v)} />;
    case 'file':
      return <FileDialog spec={spec} done={(v) => d.resolve(v)} />;
    case 'exportSpine':
      return <ExportDialog spec={spec} done={(v) => d.resolve(v)} />;
    case 'pngSequence':
      return <PngDialog spec={spec} done={(v) => d.resolve(v)} />;
    case 'pickImage':
      return <PickImageDialog spec={spec} done={(v) => d.resolve(v)} />;
    case 'shortcuts':
      return <ShortcutsDialog done={() => d.resolve(undefined)} />;
    case 'aiConnect':
      return <AiConnectDialog done={() => d.resolve(undefined)} />;
  }
}

export function DialogHost() {
  const s = useServices();
  const stack = useStore(s.dialogs, (x) => x.stack);
  const top = stack[stack.length - 1];
  return top ? <DialogView key={top.id} d={top} /> : null;
}
