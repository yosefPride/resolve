import Button from '../ui/Button';

export default function FinalCta() {
  return (
    <section className="border-t border-white/10 py-32 sm:py-40">
      {/* No card, no tint — the headline sits straight on the page black, so
          the last thing a visitor sees is the line itself rather than another
          bordered box. `font-display` is Space Grotesk (see main.css). */}
      <div className="mx-auto max-w-4xl px-4 text-center sm:px-6 lg:px-8">
        <h2 className="font-display text-4xl font-bold tracking-tight text-balance text-white sm:text-6xl lg:text-7xl">
          Ready to fix issues faster?
        </h2>
        <p className="mx-auto mt-6 max-w-md text-base text-slate-400 sm:text-lg">
          Create a team and start tracking issues in minutes.
        </p>
        <div className="mt-10 flex items-center justify-center gap-4">
          <Button to="/register" size="lg">
            Get started
          </Button>
        </div>
      </div>
    </section>
  );
}
