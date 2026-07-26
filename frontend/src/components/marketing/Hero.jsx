import heroImage from '../../assets/hero.jpeg';
import Button from '../ui/Button';

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
          <Button to="/register" size="lg">
            Get started
          </Button>
          <Button to="/login" variant="ghost" size="lg" className="border border-white/10">
            Log in
          </Button>
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
