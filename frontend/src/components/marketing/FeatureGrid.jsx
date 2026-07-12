const FEATURES = [
  {
    title: 'Strict group isolation',
    description: 'Tickets and comments belong to exactly one group. No cross-tenant access, ever.',
    icon: (
      <path strokeLinecap="round" strokeLinejoin="round" d="M12 3l7 3.5v5c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9v-5L12 3z" />
    ),
  },
  {
    title: 'Role-aware access',
    description: 'Contributor, Group Admin, and System Admin roles are enforced on every request, not just the UI.',
    icon: (
      <>
        <circle cx="12" cy="7.5" r="3" />
        <path strokeLinecap="round" d="M5.5 19c0-3.6 3-6 6.5-6s6.5 2.4 6.5 6" />
      </>
    ),
  },
  {
    title: 'Built-in workflow',
    description: 'Report, discuss, and resolve tickets in one place, with a full comment and status history.',
    icon: (
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M4 6h16M4 6v12a2 2 0 002 2h12a2 2 0 002-2V6M9 6V4h6v2M9 12h6"
      />
    ),
  },
  {
    title: 'AI-assisted triage',
    description: 'Gemini-powered suggestions summarize, categorize, and explain tickets — advisory only, never authoritative.',
    icon: (
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M12 3v3M12 18v3M4.2 6.2l2.1 2.1M17.7 15.7l2.1 2.1M3 12h3M18 12h3M4.2 17.8l2.1-2.1M17.7 8.3l2.1-2.1M12 8a4 4 0 100 8 4 4 0 000-8z"
      />
    ),
  },
  {
    title: 'Live group dashboards',
    description: 'See ticket volume, status, and AI-generated reports scoped to your active group in real time.',
    icon: (
      <path strokeLinecap="round" strokeLinejoin="round" d="M4 20V10M10 20V4M16 20v-7M4 20h16" />
    ),
  },
  {
    title: 'Secure by default',
    description: 'JWT authentication and bcrypt password hashing protect every account from day one.',
    icon: (
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M12 15v2M7 10V7a5 5 0 0110 0v3M6 10h12a1 1 0 011 1v9a1 1 0 01-1 1H6a1 1 0 01-1-1v-9a1 1 0 011-1z"
      />
    ),
  },
];

export default function FeatureGrid() {
  return (
    <section className="border-t border-white/10 bg-white/2 py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-3xl font-bold tracking-tight text-white sm:text-4xl">
            Everything you need
          </h2>
          <p className="mt-4 text-base text-slate-400">
            A focused feature set built around isolation, roles, and speed — not sprawl.
          </p>
        </div>

        <div className="mt-14 grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
          {FEATURES.map((feature) => (
            <div
              key={feature.title}
              className="rounded-2xl border border-white/10 bg-white/5 p-6 transition-all duration-200 hover:border-gray-100/40 hover:shadow-[0_0_20px_-5px_rgba(243,244,246,0.4)]"
            >
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-gray-100/30 text-gray-300">
                <svg viewBox="0 0 24 24" className="h-5 w-5" fill="none" stroke="currentColor" strokeWidth="1.5">
                  {feature.icon}
                </svg>
              </div>
              <h3 className="mt-4 text-base font-semibold text-white">{feature.title}</h3>
              <p className="mt-2 text-sm text-slate-400">{feature.description}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
