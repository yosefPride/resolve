import { Link } from 'react-router-dom';

const AUDIENCES = [
  {
    title: 'Contributors',
    description: 'Focus on fixing bugs, not managing tools.',
    points: [
      'File and track tickets in your group',
      'Comment and collaborate with your team',
      'AI-assisted context on every ticket',
    ],
  },
  {
    title: 'Group Admins',
    description: 'Full control over your group, without extra overhead.',
    points: [
      'Manage members and roles',
      'Assign and prioritize tickets',
      'AI-generated group reports',
    ],
  },
  {
    title: 'Cross-functional teams',
    description: 'Visibility into progress, without needing to touch code.',
    points: [
      'Read-only insight into ticket status',
      'Group-scoped, never cross-tenant',
      'No separate tool to learn',
    ],
  },
];

export default function AudienceCards() {
  return (
    <section className="border-t border-white/10 bg-white/2 py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-3xl font-bold tracking-tight text-white sm:text-4xl">
            Built for every team
          </h2>
          <p className="mt-4 text-base text-slate-400">
            Whichever role you play, Resolve scopes the experience to what you actually need.
          </p>
        </div>

        <div className="mt-14 grid grid-cols-1 gap-6 lg:grid-cols-3">
          {AUDIENCES.map((audience) => (
            <div
              key={audience.title}
              className="rounded-2xl border border-gray-100/20 bg-white/5 p-6"
            >
              <h3 className="text-lg font-semibold text-white">{audience.title}</h3>
              <p className="mt-2 text-sm text-slate-400">{audience.description}</p>
              <ul className="mt-5 space-y-2.5">
                {audience.points.map((point) => (
                  <li key={point} className="flex items-start gap-2 text-sm text-slate-300">
                    <svg
                      viewBox="0 0 24 24"
                      className="mt-0.5 h-4 w-4 shrink-0 text-gray-100"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                    >
                      <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
                    </svg>
                    {point}
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>

        <div className="mt-16 rounded-3xl border border-white/10 bg-white/5 px-6 py-14 text-center sm:px-12">
          <h2 className="text-2xl font-bold tracking-tight text-white sm:text-3xl">
            Ready to fix bugs faster?
          </h2>
          <p className="mx-auto mt-3 max-w-md text-sm text-slate-400">
            Create a group and start tracking tickets in minutes.
          </p>
          <div className="mt-7 flex items-center justify-center gap-4">
            <Link
              to="/register"
              className="rounded-full bg-white px-6 py-3 text-sm font-semibold text-black transition-all duration-200 hover:bg-black hover:ring-1 hover:ring-white hover:text-white disabled:cursor-not-allowed disabled:bg-white/50 disabled:text-black/50"
            >
              Get started
            </Link>
            <Link
              to="/login"
              className="rounded-full px-6 py-3 text-sm font-medium text-slate-300 transition-colors hover:bg-white/10 hover:text-white"
            >
              Log in
            </Link>
          </div>
        </div>
      </div>
    </section>
  );
}
