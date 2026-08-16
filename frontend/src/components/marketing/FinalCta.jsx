import Button from '../ui/Button';

export default function FinalCta() {
  return (
    <section className="border-t border-white/10 py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="rounded-3xl border border-white/10 bg-white/5 px-6 py-14 text-center sm:px-12">
          <h2 className="text-2xl font-bold tracking-tight text-white sm:text-3xl">
            Ready to fix issues faster?
          </h2>
          <p className="mx-auto mt-3 max-w-md text-sm text-slate-400">
            Create a team and start tracking issues in minutes.
          </p>
          <div className="mt-7 flex items-center justify-center gap-4">
            <Button to="/register" size="lg">
              Get started
            </Button>
          </div>
        </div>
      </div>
    </section>
  );
}
