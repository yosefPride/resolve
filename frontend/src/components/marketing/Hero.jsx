import Button from '../ui/Button';
import ProductDemo from './demo/ProductDemo';

export default function Hero() {
  return (
    <section className="relative overflow-hidden">
      <div className="max-w-7xl mx-auto px-4 pt-20 pb-16 text-left sm:px-6 sm:pt-28 lg:px-8">

        <h1 className="mt-6 max-w-3xl text-4xl font-bold tracking-tight text-white sm:text-5xl lg:text-6xl">
          Ship fixes, not spreadsheets.
        </h1>

        <p className="mt-5 max-w-xl text-base text-slate-400 sm:text-lg">
          Resolve gives every team a single home for tracking bugs — with AI
          assistance that speeds up triage without ever touching your data.
        </p>

        <div className="mt-8 flex items-center gap-4">
          <Button to="/register" size="lg">
            Get started
          </Button>
          <Button to="/login" variant="ghost" size="lg" className="border border-white/10">
            Log in
          </Button>
        </div>
      </div>

      <div className="relative mx-auto  px-4 pb-24 sm:px-6 lg:px-8">
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-x-0 top-20 -z-10 h-225 w-screen bg-linear-to-b from-white/10 via-white/20 to-white/60 blur-[180px]
          "
        />
        <div className="max-w-7xl mx-auto">
          <ProductDemo />
        </div>
      </div>
    </section>
  );
}
