import { useCallback, useEffect, useRef, useState } from 'react';

// HexagonBackground — honeycomb lattice that lights a cell under the cursor and
// lets it fade back out over a second. Ported from Animate UI
// (https://animate-ui.com/docs/components/backgrounds/hexagon, MIT); the
// upstream source is TypeScript + Next's 'use client' + a `cn` helper, all
// stripped here, and the neutral light/dark palette is replaced with this app's
// black-on-black one.
//
// Two deviations worth knowing about:
//   - upstream sizes the grid from window.innerWidth/innerHeight, which is
//     wasteful in a short container (a footer would build a full viewport's
//     worth of rows and clip them). This measures the element instead.
//   - upstream injects `:root { --hexagon-margin }` into a global <style>. The
//     variable is set on the container here so the value stays scoped to it.
//
// The root deliberately sets no position or size utilities: Tailwind emits
// `.relative` after `.absolute`, so a `relative` baked in here would beat the
// `absolute` a caller passes via className (equal specificity, source order
// decides) and the layer would collapse to zero height in an auto-height
// parent. Callers own the positioning — see Footer.
//
// Each cell is a clipped hexagon (::before) with a second, inset hexagon on top
// (::after). Both are black at rest, so only the gaps between cells show the
// container behind them; on hover ::before brightens and reads as a ring.
export default function HexagonBackground({
  hexagonSize = 75,
  hexagonMargin = 3,
  className = '',
  children,
  ...props
}) {
  const containerRef = useRef(null);
  const [grid, setGrid] = useState({ rows: 0, columns: 0 });

  const hexagonWidth = hexagonSize;
  const hexagonHeight = hexagonSize * 1.1;
  const rowSpacing = hexagonSize * 0.8;
  // Rows overlap so the points interlock; the offset is tuned to hexagonSize.
  const computedMarginTop = -36 - 0.275 * (hexagonSize - 100) + hexagonMargin;
  const oddRowMarginLeft = -(hexagonSize / 2);
  const evenRowMarginLeft = hexagonMargin / 2;

  const measure = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;
    setGrid({
      rows: Math.ceil(el.clientHeight / rowSpacing) + 1,
      columns: Math.ceil(el.clientWidth / hexagonWidth) + 1,
    });
  }, [rowSpacing, hexagonWidth]);

  useEffect(() => {
    measure();
    const observer = new ResizeObserver(measure);
    if (containerRef.current) observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, [measure]);

  return (
    <div
      ref={containerRef}
      style={{ '--hexagon-margin': `${hexagonMargin}px` }}
      className={`overflow-hidden bg-white/10 ${className}`}
      {...props}
    >
      <div className="absolute top-0 left-0 size-full overflow-hidden">
        {Array.from({ length: grid.rows }).map((_, rowIndex) => (
          <div
            key={`row-${rowIndex}`}
            style={{
              marginTop: computedMarginTop,
              marginLeft: ((rowIndex + 1) % 2 === 0 ? evenRowMarginLeft : oddRowMarginLeft) - 10,
            }}
            className="inline-flex"
          >
            {Array.from({ length: grid.columns }).map((_, colIndex) => (
              <div
                key={`hexagon-${rowIndex}-${colIndex}`}
                style={{ width: hexagonWidth, height: hexagonHeight, marginLeft: hexagonMargin }}
                className="relative [clip-path:polygon(50%_0%,_100%_25%,_100%_75%,_50%_100%,_0%_75%,_0%_25%)] before:absolute before:top-0 before:left-0 before:h-full before:w-full before:bg-black before:transition-all before:duration-1000 before:content-[''] after:absolute after:inset-[var(--hexagon-margin)] after:bg-black after:[clip-path:polygon(50%_0%,_100%_25%,_100%_75%,_50%_100%,_0%_75%,_0%_25%)] after:content-[''] hover:before:bg-white/30 hover:before:duration-0"
              />
            ))}
          </div>
        ))}
      </div>
      {children}
    </div>
  );
}
