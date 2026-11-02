/** The Overview's progress visual: a piece of plain weave. See docs/DESIGN.md. */

export interface Thread {
  title: string;
  done: boolean;
}

const PITCH = 20;
const WIDTH = 12;
const GAP = 1.5;
const MIN_WARP = 9;
const MIN_WEFT = 5;
const MAX_WARP = 16;
const MAX_WEFT = 9;

type Strand = Thread & { ghost?: boolean };

/** Fits items onto the loom. Spare positions become faint threads, so the cloth keeps its shape. */
export function layOut(items: Thread[], minimum: number, maximum: number): Strand[] {
  if (items.length > maximum) {
    const done = items.filter((item) => item.done).length;
    const filled = Math.round((done / items.length) * maximum);
    return Array.from({ length: maximum }, (_, index) => ({
      title: `${items.length} items`,
      done: index < filled,
    }));
  }
  const size = Math.max(minimum, items.length);
  const spare = size - items.length;
  const before = Math.floor(spare / 2);
  const ghost = (): Strand => ({ title: "", done: false, ghost: true });
  return [
    ...Array.from({ length: before }, ghost),
    ...items,
    ...Array.from({ length: spare - before }, ghost),
  ];
}

const color = (thread: Strand, done: string) =>
  thread.ghost ? "var(--thread-ghost)" : thread.done ? done : "var(--thread-idle)";

export function Weave({
  tasks,
  habits,
  background = "var(--window)",
  label,
}: {
  tasks: Thread[];
  habits: Thread[];
  background?: string;
  label: string;
}) {
  const warp = layOut(habits, MIN_WARP, MAX_WARP);
  const weft = layOut(tasks, MIN_WEFT, MAX_WEFT);
  const width = warp.length * PITCH;
  const height = weft.length * PITCH;
  const center = (index: number) => (index + 0.5) * PITCH;
  const warpColor = (thread: Strand) => color(thread, "var(--thread-warp)");
  const weftColor = (thread: Strand) => color(thread, "var(--color-accent)");
  const transition = { transition: "fill 300ms var(--ease-weave), stroke 300ms var(--ease-weave)" };

  return (
    <svg
      role="img"
      aria-label={label}
      viewBox={`0 0 ${width} ${height}`}
      width={width}
      height={height}
      className="max-h-[132px] w-auto shrink-0"
    >
      {warp.map((thread, column) => (
        <line
          key={column}
          x1={center(column)}
          x2={center(column)}
          y1={WIDTH / 2}
          y2={height - WIDTH / 2}
          strokeWidth={WIDTH}
          strokeLinecap="round"
          stroke={warpColor(thread)}
          style={transition}
        >
          {thread.title && <title>{thread.title}</title>}
        </line>
      ))}
      {weft.map((thread, row) => (
        <line
          key={row}
          y1={center(row)}
          y2={center(row)}
          x1={WIDTH / 2}
          x2={width - WIDTH / 2}
          strokeWidth={WIDTH}
          strokeLinecap="round"
          stroke={weftColor(thread)}
          style={transition}
        >
          {thread.title && <title>{thread.title}</title>}
        </line>
      ))}
      {/* Each crossing lifts one thread over the other, with a sliver of background either side. */}
      {weft.map((across, row) =>
        warp.map((down, column) => {
          const x = center(column) - WIDTH / 2;
          const y = center(row) - WIDTH / 2;
          const warpOver = (row + column) % 2 === 1;
          return warpOver ? (
            <g key={`${row}-${column}`}>
              <rect x={x - GAP} y={y} width={WIDTH + 2 * GAP} height={WIDTH} fill={background} />
              <rect x={x} y={y - 1} width={WIDTH} height={WIDTH + 2} fill={warpColor(down)} style={transition} />
            </g>
          ) : (
            <g key={`${row}-${column}`}>
              <rect x={x} y={y - GAP} width={WIDTH} height={WIDTH + 2 * GAP} fill={background} />
              <rect x={x - 1} y={y} width={WIDTH + 2} height={WIDTH} fill={weftColor(across)} style={transition} />
            </g>
          );
        }),
      )}
    </svg>
  );
}
