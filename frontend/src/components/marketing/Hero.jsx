import { Link } from 'react-router-dom';
import heroImage from '../../assets/hero.jpeg';

export default function Hero() {
  return (
    <section className="relative overflow-hidden">
      <div className="mx-auto max-w-7xl px-4 pt-20 pb-16 text-center sm:px-6 sm:pt-28 lg:px-8">

        <h1 className="mx-auto mt-6 max-w-3xl text-4xl font-bold tracking-tight text-white sm:text-5xl lg:text-6xl">
          Ship fixes, not spreadsheets.
        </h1>

        <p className="mx-auto mt-5 max-w-xl text-base text-slate-400 sm:text-lg">
          Resolve gives every team a single home for tracking bugs — with AI
          assistance that speeds up triage without ever touching your data.
        </p>

        <div className="mt-8 flex items-center justify-center gap-4">
          <Link
            to="/register"
            className="rounded-full bg-white px-6 py-3 text-sm font-semibold text-black transition-all duration-200 hover:bg-black hover:ring-1 hover:ring-white hover:text-white disabled:cursor-not-allowed disabled:bg-white/50 disabled:text-black/50"
          >
            Get started
          </Link>
          <Link
            to="/login"
            className="rounded-full border border-white/10 px-6 py-3 text-sm font-medium text-slate-300 transition-colors hover:bg-white/10 hover:text-white"
          >
            Log in
          </Link>
        </div>
      </div>

      <div className="relative mx-auto max-w-6xl px-4 pb-24 sm:px-6 lg:px-8">
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-x-8 top-8 -z-10 h-[80%] rounded-4xl bg-gray-100/40 blur-3xl"
        />
        <div className="overflow-hidden rounded-2xl border border-white/10 shadow-2xl shadow-black/50">
          <img src={heroImage} alt="Resolve dashboard preview" className="w-full object-cover" />
        </div>
      </div>
    </section>
  );
}
