/**
 * 16px outline icons: 1.5px stroke, round caps and joins, drawn in
 * currentColor (Afterglow iconography). Decorative by default; give the
 * surrounding button an aria-label.
 */

const PATHS = {
  library: (
    <>
      <rect x="2.5" y="2.5" width="4.5" height="4.5" rx="1" />
      <rect x="9" y="2.5" width="4.5" height="4.5" rx="1" />
      <rect x="2.5" y="9" width="4.5" height="4.5" rx="1" />
      <rect x="9" y="9" width="4.5" height="4.5" rx="1" />
    </>
  ),
  bell: (
    <>
      <path d="M4 7a4 4 0 0 1 8 0v2.5l1 2H3l1-2z" />
      <path d="M6.5 13.5a1.5 1.5 0 0 0 3 0" />
    </>
  ),
  settings: (
    <>
      <circle cx="8" cy="8" r="2" />
      <path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4" />
    </>
  ),
  search: (
    <>
      <circle cx="7" cy="7" r="4.5" />
      <path d="M10.5 10.5 14 14" />
    </>
  ),
  play: <path d="M5 3.2v9.6a.5.5 0 0 0 .76.43l7.6-4.8a.5.5 0 0 0 0-.86l-7.6-4.8A.5.5 0 0 0 5 3.2z" />,
  plus: <path d="M8 3v10M3 8h10" />,
  refresh: (
    <>
      <path d="M13 8a5 5 0 1 1-1.5-3.55" />
      <path d="M13 2.5v3h-3" />
    </>
  ),
  image: (
    <>
      <rect x="2" y="3" width="12" height="10" rx="1.5" />
      <circle cx="6" cy="6.5" r="1" />
      <path d="m2.5 12 3.5-3.5 3 3 2-2 2.5 2.5" />
    </>
  ),
  folder: <path d="M2 4.5A1.5 1.5 0 0 1 3.5 3h3l1.5 1.5h4.5A1.5 1.5 0 0 1 14 6v5.5a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 2 11.5z" />,
  grid: (
    <>
      <rect x="2.5" y="2.5" width="4.5" height="4.5" rx="1" />
      <rect x="9" y="2.5" width="4.5" height="4.5" rx="1" />
      <rect x="2.5" y="9" width="4.5" height="4.5" rx="1" />
      <rect x="9" y="9" width="4.5" height="4.5" rx="1" />
    </>
  ),
  list: <path d="M5.5 4h8M5.5 8h8M5.5 12h8M2.5 4h0M2.5 8h0M2.5 12h0" />,
  back: <path d="M10 3.5 5.5 8l4.5 4.5" />,
  close: <path d="M4 4l8 8M12 4l-8 8" />,
  lock: (
    <>
      <rect x="3.5" y="7" width="9" height="6.5" rx="1.5" />
      <path d="M5.5 7V5a2.5 2.5 0 0 1 5 0v2" />
    </>
  ),
  bolt: <path d="M9 1.5 3.5 9H8l-1 5.5L12.5 7H8z" />,
  target: (
    <>
      <circle cx="8" cy="8" r="5.5" />
      <circle cx="8" cy="8" r="2" />
    </>
  ),
  link: (
    <>
      <path d="M6.5 9.5a2.5 2.5 0 0 0 3.5 0l2.5-2.5a2.5 2.5 0 0 0-3.5-3.5L8.5 4" />
      <path d="M9.5 6.5a2.5 2.5 0 0 0-3.5 0L3.5 9a2.5 2.5 0 0 0 3.5 3.5l.5-.5" />
    </>
  ),
  save: (
    <>
      <path d="M3 3.5A1.5 1.5 0 0 1 4.5 2h6L13 4.5v8a1.5 1.5 0 0 1-1.5 1.5h-7A1.5 1.5 0 0 1 3 12.5z" />
      <path d="M5.5 2v3h4V2M5.5 14v-4h5v4" />
    </>
  ),
  stop: <rect x="4" y="4" width="8" height="8" rx="1.5" />,
} as const;

export type IconName = keyof typeof PATHS;

export default function Icon({ name, size = 16 }: { name: IconName; size?: number }) {
  return (
    <svg className="ag-icon" viewBox="0 0 16 16" width={size} height={size} aria-hidden="true">
      {PATHS[name]}
    </svg>
  );
}
