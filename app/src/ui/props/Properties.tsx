// 속성 패널(오른쪽 아래): 선택 항목 종류별 필드.
import { useServices } from '../../services';
import { useStore } from '../../state/store';
import { AttachmentProps } from './AttachmentProps';
import { BoneProps } from './BoneProps';
import { ConstraintProps } from './ConstraintProps';
import { AnimationProps, EventProps, ImageProps, SkeletonProps, SkinProps } from './OtherProps';
import { SlotProps } from './SlotProps';

function Body() {
  const s = useServices();
  const sel = useStore(s.ui, (u) => u.selection[u.selection.length - 1]);
  const sk = useStore(s.doc, (d) => d.project.skeleton);
  if (!sel) return <div className="props-empty">선택한 항목이 없습니다</div>;
  switch (sel.kind) {
    case 'skeleton':
      return <SkeletonProps />;
    case 'bone': {
      const b = sk.bones.find((x) => x.name === sel.name);
      return b ? <BoneProps key={b.name} bone={b} /> : null;
    }
    case 'slot': {
      const x = sk.slots.find((y) => y.name === sel.name);
      return x ? <SlotProps key={x.name} slot={x} /> : null;
    }
    case 'attachment': {
      const a = sk.skins.find((k) => k.name === sel.skin)?.attachments[sel.slot ?? '']?.[sel.name];
      return a ? <AttachmentProps key={`${sel.skin}/${sel.slot}/${sel.name}`} sel={sel} att={a} /> : null;
    }
    case 'constraint': {
      const c = sk.constraints.find((x) => x.name === sel.name);
      return c ? <ConstraintProps key={c.name} c={c} /> : null;
    }
    case 'skin': {
      const k = sk.skins.find((x) => x.name === sel.name);
      return k ? <SkinProps key={k.name} skin={k} /> : null;
    }
    case 'event': {
      const e = sk.events.find((x) => x.name === sel.name);
      return e ? <EventProps key={e.name} ev={e} /> : null;
    }
    case 'animation': {
      const a = sk.animations.find((x) => x.name === sel.name);
      return a ? <AnimationProps key={a.name} anim={a} /> : null;
    }
    case 'image':
      return <ImageProps name={sel.name} />;
  }
}

export function Properties() {
  return (
    <div className="props-panel panel" data-testid="properties">
      <div className="panel-title">속성</div>
      <div className="props-body">
        <Body />
      </div>
    </div>
  );
}
