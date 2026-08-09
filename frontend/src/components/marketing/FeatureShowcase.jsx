import linksImg from '../../assets/links.png';
import activityImg from '../../assets/activity.png';
import dashboardImg from '../../assets/dashboard.png';

const ITEMS = [
  {
    image: linksImg,
    title: 'Link related issues',
    description: 'Connect issues to each other and attach outside references, right from the ticket.',
  },
  {
    image: activityImg,
    title: 'Track every change',
    description: 'A full activity trail — status changes, edits, links — all timestamped and attributed.',
  },
  {
    image: dashboardImg,
    title: 'Your teams, at a glance',
    description: 'Open issues, critical items, and recent activity across every team you belong to.',
  },
];

export default function FeatureShowcase() {
  return (
    <section className="border-t border-white/10 py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="max-w-2xl text-left">
          <h2 className="text-3xl font-bold tracking-tight text-white sm:text-4xl">A closer look</h2>
          <p className="mt-4 text-base text-slate-400">
            From a team's shared dashboard down to a single issue's history — a look at the details.
          </p>
        </div>

        <div className="mt-14 grid grid-cols-1 gap-10 sm:grid-cols-3 sm:gap-8">
          {ITEMS.map((item) => (
            <div key={item.title}>
              <img
                src={item.image}
                alt={item.title}
                className="w-full rounded-2xl border border-white/10 shadow-2xl shadow-black/50"
              />
              <h3 className="mt-5 text-base font-semibold text-white">{item.title}</h3>
              <p className="mt-2 text-sm text-slate-400">{item.description}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
