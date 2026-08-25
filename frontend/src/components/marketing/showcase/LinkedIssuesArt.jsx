// Two issue cards linking up, then a third joining them — the marketing
// picture of what LinksSection does inside a ticket.
//
// All SVG: the connectors have to start and end exactly on the cards, and
// aligning an absolutely-positioned HTML overlay to an SVG line is a much
// worse job than just drawing the cards as <rect>s too.
//
// `active` only ever ADDS the animation. The resting state is the finished
// picture, so a tile that never animates — off screen, or under
// prefers-reduced-motion — still shows three linked issues rather than a
// half-drawn one.
export default function LinkedIssuesArt({ active }) {
  return (
    <svg viewBox="0 0 200 200" fill="none" className="h-full w-full">
      {/* Connectors sit behind the cards so the ends tuck under the edges. */}
      <g stroke="currentColor" strokeWidth="1.5" className="text-white/45">
        <path
          d="M74 62 H126"
          pathLength="1"
          strokeDasharray="1"
          className={active ? 'animate-link-draw' : ''}
        />
        <path
          d="M100 78 V138"
          pathLength="1"
          strokeDasharray="1"
          className={active ? 'animate-link-draw' : ''}
          style={active ? { animationDelay: '0.5s' } : undefined}
        />
      </g>

      <IssueCard x={26} y={46} />
      <IssueCard x={126} y={46} />
      {/* The third card arrives with its connector rather than sitting there
          waiting for it. */}
      <g
        className={active ? 'animate-node-in' : ''}
        style={active ? { animationDelay: '0.5s' } : undefined}
      >
        <IssueCard x={76} y={138} />
      </g>
    </svg>
  );
}

// A ticket rendered small: a rounded frame with a title bar and a shorter
// second line, the way the real ticket cards read at a glance.
function IssueCard({ x, y }) {
  return (
    <g transform={`translate(${x} ${y})`}>
      <rect
        width="48"
        height="32"
        rx="6"
        className="fill-white/5 stroke-white/15"
        strokeWidth="1"
      />
      <rect x="8" y="10" width="26" height="3" rx="1.5" className="fill-white/50" />
      <rect x="8" y="18" width="16" height="3" rx="1.5" className="fill-white/20" />
    </g>
  );
}
