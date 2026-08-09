// Shared shell for the landing page's feature spotlights: a short left-
// aligned heading + one-line body, then the demo visual at full width
// beneath it — same shape as the Hero's own heading-above-demo layout, and
// the same outer container (max-w-7xl) so every spotlight's visual lines up
// with the hero demo's width exactly. `tinted` alternates the section
// background so consecutive spotlights stay visually distinct.
export default function FeatureSpotlight({ title, description, tinted = false, children }) {
  return (
    <section className={`border-t border-white/10 py-20 sm:py-24 ${tinted ? 'bg-white/2' : ''}`}>
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="max-w-2xl text-left">
          <h2 className="text-3xl font-bold tracking-tight text-white sm:text-4xl">{title}</h2>
          <p className="mt-4 text-base text-slate-400">{description}</p>
        </div>
        <div className="mt-14">{children}</div>
      </div>
    </section>
  );
}
