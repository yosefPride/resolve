import useInView from '../../hooks/useInView';
import LinkedIssuesArt from './showcase/LinkedIssuesArt';
import ActivityTrailArt from './showcase/ActivityTrailArt';
import TeamGlanceArt from './showcase/TeamGlanceArt';

const ITEMS = [
  {
    Art: LinkedIssuesArt,
    title: 'Link related issues',
    description: 'Connect issues to each other and attach outside references, right from the ticket.',
  },
  {
    Art: ActivityTrailArt,
    title: 'Track every change',
    description: 'A full activity trail — status changes, edits, links — all timestamped and attributed.',
  },
  {
    Art: TeamGlanceArt,
    title: 'Your teams, at a glance',
    description: 'Open issues, critical items, and recent activity across every team you belong to.',
  },
];

export default function FeatureShowcase() {
  return (
    <section className="border-t border-white/10 py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        {/* One statement rather than a heading over a paragraph — the three
            cards below already do the explaining. */}
        <p className="max-w-3xl text-2xl font-medium tracking-tight text-balance text-white sm:text-3xl">
          Issues that link to each other, a trail of every change, and a dashboard that keeps up
          with both.
        </p>

        <div className="mt-14 grid grid-cols-1 gap-10 sm:grid-cols-3 sm:gap-8">
          {ITEMS.map((item) => (
            <FeatureCard key={item.title} {...item} />
          ))}
        </div>
      </div>
    </section>
  );
}

// The animation loops, so it only starts once the tile has scrolled into view —
// there's no reason to keep three of them running above the fold all session.
function FeatureCard({ Art, title, description }) {
  const [ref, inView] = useInView();

  return (
    <div
      ref={ref}
      data-showcase-art
      className="relative flex aspect-square flex-col overflow-hidden p-6 text-white lg:aspect-[5/6]"
    >
      {/* An open frame: the top and bottom rules run the full width, while the
          sides are only stubs hanging off each corner, leaving the middle of
          each side open. Drawn as six 1px bars rather than borders on the card
          so each run's length is set independently. */}
      <span className="pointer-events-none absolute inset-x-0 top-0 h-px bg-white/25" />
      <span className="pointer-events-none absolute inset-x-0 bottom-0 h-px bg-white/25" />
      <span className="pointer-events-none absolute top-0 left-0 h-20 w-px bg-white/25" />
      <span className="pointer-events-none absolute top-0 right-0 h-20 w-px bg-white/25" />
      <span className="pointer-events-none absolute bottom-0 left-0 h-20 w-px bg-white/25" />
      <span className="pointer-events-none absolute right-0 bottom-0 h-20 w-px bg-white/25" />

      {/* min-h-0 lets the art shrink to whatever the copy leaves it, instead of
          pushing the title out of a card whose height is pinned square. */}
      <div className="min-h-0 grow">
        <Art active={inView} />
      </div>
      <h3 className="mt-5 text-base font-semibold text-white">{title}</h3>
      <p className="mt-2 text-sm text-slate-400">{description}</p>
    </div>
  );
}
