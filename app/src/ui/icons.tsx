// 직접 그린 단순 SVG 아이콘.
import type { ReactElement } from 'react';

const P = { fill: 'none', stroke: 'currentColor', strokeWidth: 1.6, strokeLinecap: 'round', strokeLinejoin: 'round' } as const;

const paths: Record<string, ReactElement> = {
  pose: (
    <>
      <circle cx="8" cy="4" r="2" {...P} />
      <path d="M8 6v5M4 8l4 1 4-1M8 11l-3 4M8 11l3 4" {...P} />
    </>
  ),
  create: (
    <>
      <path d="M3 13L11 5" {...P} />
      <path d="M3 13l1.5-4L8 12z" {...P} />
      <path d="M12 1v4M10 3h4" {...P} />
    </>
  ),
  weights: (
    <>
      <path d="M10 2l4 4-6 6-3 1 1-3z" {...P} />
      <path d="M2 14c1.5 0 2-1 2-2" {...P} />
    </>
  ),
  rotate: (
    <>
      <path d="M13 8a5 5 0 1 1-2-4" {...P} />
      <path d="M11 1v3h3" {...P} />
    </>
  ),
  translate: <path d="M8 1v14M1 8h14M8 1L6 3M8 1l2 2M8 15l-2-2M8 15l2-2M1 8l2-2M1 8l2 2M15 8l-2-2M15 8l-2 2" {...P} />,
  scale: (
    <>
      <rect x="2" y="8" width="6" height="6" {...P} />
      <path d="M8 8l6-6M10 2h4v4" {...P} />
    </>
  ),
  shear: <path d="M5 3h9l-3 10H2z" {...P} />,
  eye: (
    <>
      <path d="M1 8s2.5-5 7-5 7 5 7 5-2.5 5-7 5-7-5-7-5z" {...P} />
      <circle cx="8" cy="8" r="2" {...P} />
    </>
  ),
  eyeOff: (
    <>
      <path d="M1 8s2.5-5 7-5 7 5 7 5-2.5 5-7 5-7-5-7-5z" {...P} opacity={0.35} />
      <path d="M2 14L14 2" {...P} />
    </>
  ),
  first: <path d="M4 3v10M13 3L6 8l7 5z" {...P} />,
  last: <path d="M12 3v10M3 3l7 5-7 5z" {...P} />,
  play: <path d="M4 2l10 6-10 6z" {...P} />,
  reverse: <path d="M12 2L2 8l10 6z" {...P} />,
  stop: <rect x="4" y="4" width="8" height="8" {...P} />,
  loop: (
    <>
      <path d="M2 7a4 4 0 0 1 4-4h6M14 9a4 4 0 0 1-4 4H4" {...P} />
      <path d="M10 1l2 2-2 2M6 11l-2 2 2 2" {...P} />
    </>
  ),
  bone: <path d="M2 14L5 7l9-5-5 9z" {...P} />,
  slot: <rect x="3" y="3" width="10" height="10" rx="2" {...P} />,
  attachment: (
    <>
      <rect x="2" y="3" width="12" height="10" {...P} />
      <path d="M2 11l4-4 3 3 2-2 3 3" {...P} />
    </>
  ),
  constraint: (
    <>
      <circle cx="4" cy="12" r="2" {...P} />
      <circle cx="12" cy="4" r="2" {...P} />
      <path d="M5.5 10.5l5-5" {...P} strokeDasharray="2 2" />
    </>
  ),
  skin: <path d="M3 3h10v4l-5 6-5-6z" {...P} />,
  event: <path d="M8 1l2 5h5l-4 3 2 5-5-3-5 3 2-5-4-3h5z" {...P} />,
  animation: (
    <>
      <rect x="2" y="3" width="12" height="10" rx="1" {...P} />
      <path d="M6 6l4 2-4 2z" {...P} />
    </>
  ),
  skeleton: (
    <>
      <circle cx="8" cy="8" r="6" {...P} />
      <circle cx="8" cy="8" r="1.5" {...P} />
    </>
  ),
  image: (
    <>
      <rect x="2" y="3" width="12" height="10" {...P} />
      <circle cx="6" cy="6.5" r="1.2" {...P} />
    </>
  ),
  folder: <path d="M2 4h4l1 2h7v7H2z" {...P} />,
  key: <path d="M8 2l5 6-5 6-5-6z" {...P} fill="currentColor" />,
  chevron: <path d="M6 4l4 4-4 4" {...P} />,
  chevronDown: <path d="M4 6l4 4 4-4" {...P} />,
};

export function Icon({ name, size = 16, title }: { name: keyof typeof paths | string; size?: number; title?: string }) {
  return (
    <svg className="icon" width={size} height={size} viewBox="0 0 16 16" aria-hidden={title ? undefined : true}>
      {title && <title>{title}</title>}
      {paths[name] ?? paths.slot}
    </svg>
  );
}
