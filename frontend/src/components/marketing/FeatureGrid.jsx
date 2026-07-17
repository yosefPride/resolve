import { ShieldCheck, UserCheck, ClipboardList, Sparkles, BarChart3, Lock } from 'lucide-react';

const FEATURES = [
  {
    title: 'Strict team isolation',
    description: 'Tickets and comments belong to exactly one team. No cross-tenant access, ever.',
    icon: ShieldCheck,
  },
  {
    title: 'Role-aware access',
    description: 'Contributor, Team Admin, and System Admin roles are enforced on every request, not just the UI.',
    icon: UserCheck,
  },
  {
    title: 'Built-in workflow',
    description: 'Report, discuss, and resolve tickets in one place, with a full comment and status history.',
    icon: ClipboardList,
  },
  {
    title: 'AI-assisted triage',
    description: 'Gemini-powered suggestions summarize, categorize, and explain tickets — advisory only, never authoritative.',
    icon: Sparkles,
  },
  {
    title: 'Live team dashboards',
    description: 'See ticket volume, status, and AI-generated reports scoped to your active team in real time.',
    icon: BarChart3,
  },
  {
    title: 'Secure by default',
    description: 'JWT authentication and bcrypt password hashing protect every account from day one.',
    icon: Lock,
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
                <feature.icon className="h-5 w-5" strokeWidth={1.5} />
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
