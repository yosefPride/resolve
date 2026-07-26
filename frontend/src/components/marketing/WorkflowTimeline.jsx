const STEPS = [
  {
    title: 'Report',
    description: 'A contributor files an issue inside their team, with full context attached.',
  },
  {
    title: 'Triage',
    description: 'AI suggests priority and category as an advisory hint.',
  },
  {
    title: 'Discuss & assign',
    description: 'The team comments, updates status, and assigns the issue to a developer.',
  },
  {
    title: 'Resolve',
    description: 'The issue closes, and its history stays with the team for future reference.',
  },
];

export default function WorkflowTimeline() {
  return (
    <section className="border-t border-white/10 py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-3xl font-bold tracking-tight text-white sm:text-4xl">
            From bug report to resolution
          </h2>
          <p className="mt-4 text-base text-slate-400">
            One workflow, scoped to your team, from the first report to the final fix.
          </p>
        </div>

        <div className="mt-16 grid grid-cols-1 gap-10 sm:grid-cols-2 lg:grid-cols-4 lg:gap-6">
          {STEPS.map((step, index) => (
            <div key={step.title} className="relative">
              {index < STEPS.length - 1 && (
                <div
                  aria-hidden="true"
                  className="absolute top-5 left-1/2 hidden h-px w-full bg-linear-to-r from-gray-100/40 to-transparent lg:block"
                />
              )}

              <div className="relative flex items-center gap-3 lg:flex-col lg:items-start lg:gap-0">
                <span className="relative z-10 flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-gray-100/40 bg-neutral-950 text-sm font-semibold text-gray-300 shadow-[0_0_20px_-5px_rgba(255,255,255,0.4)]">
                  {index + 1}
                </span>
                <h3 className="text-base font-semibold text-white lg:mt-4">{step.title}</h3>
              </div>

              <p className="mt-2 text-sm text-slate-400 lg:mt-2">{step.description}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
