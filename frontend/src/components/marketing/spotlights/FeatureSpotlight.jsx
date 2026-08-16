// Shared shell for the landing page's feature spotlights. Default `layout`
// ("stacked") is a short left-aligned heading + one-line body, then the demo
// visual at full width beneath it — same shape as the Hero's own
// heading-above-demo layout. `layout="side"` instead places the demo and the
// heading/body side by side (demo left, text right) for spotlights where the
// visual is compact enough to share a row. Both share the same outer
// container (max-w-7xl) so every spotlight's visual lines up with the hero
// demo's width exactly. `tinted` alternates the section background so
// consecutive spotlights stay visually distinct.
export default function FeatureSpotlight({ title, description, tinted = false, layout = 'stacked', children }) {
  const text = (
    <div className="max-w-xl text-left">
      <h2 className="text-3xl font-bold tracking-tight text-white sm:text-4xl">{title}</h2>
      <p className="mt-4 text-base text-slate-400">{description}</p>
    </div>
  );

  return (
    <section className={`border-t border-white/10 py-20 sm:py-24 ${tinted ? 'bg-white/2' : ''}`}>
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        {layout === 'side' ? (
          <div className="grid items-center gap-10 lg:grid-cols-2 lg:gap-16">
            <div>{children}</div>
            {text}
          </div>
        ) : (
          <>
            {text}
            <div className="mt-14">{children}</div>
          </>
        )}
      </div>
    </section>
  );
}
