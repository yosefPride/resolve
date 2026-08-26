import { lazy, Suspense } from 'react';
import Button from '../ui/Button';
// import ProductDemo from './demo/ProductDemo';

// Lazy so three.js stays out of the landing page's initial bundle — the hero
// renders text-first and the beams fade in when their chunk lands.
const Beams = lazy(() => import('./background/Beams'));

// `isolate` on the section keeps the -z-10 background layer in a stacking
// context of its own. Without it it belongs to the root context, where it
// paints beneath the layout's bg-black and disappears.
export default function Hero() {
  return (
    <section className="relative isolate h-screen overflow-hidden">
      {/* Full-bleed light beams. The canvas paints its own opaque black, which
          matches the layout background, so the hero's bottom edge blends into
          the rest of the page with no seam. The fallback is a plain box of the
          same size so the copy doesn't jump when the chunk lands. */}
      <Suspense fallback={<div aria-hidden="true" className="absolute inset-0 -z-10" />}>
        <div aria-hidden="true" className="absolute inset-0 -z-10">
          <Beams beamWidth={3} beamHeight={20} beamNumber={16} lightColor="#ffffff" speed={2} noiseIntensity={1.6} scale={0.2} rotation={30} />
        </div>
      </Suspense>

      {/* Scrim: the beams brighten toward the middle of the canvas, which is
          exactly where the copy sits now, so the wash is centred on it and
          fades out toward both edges. */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 -z-10 bg-radial from-black/80 via-black/40 to-transparent"
      />

      <div className="relative mx-auto flex h-full max-w-7xl items-center justify-center px-4 text-center sm:px-6 lg:px-8">
        <div className="mx-auto max-w-3xl">
          <h1 className="text-4xl font-bold tracking-tight text-white sm:text-5xl lg:text-6xl">
            Track. Discuss. Resolve.
          </h1>

          <p className="mx-auto mt-5 max-w-xl text-base text-slate-400 sm:text-lg">
            Resolve combines issue tracking and team management — powered by AI where it counts.
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

        {/* Product demo temporarily swapped out for the beams background.
            Restoring it means putting the two-column grid back — the demo needs
            the full width of its column.
        <ProductDemo />
        */}
      </div>
    </section>
  );
}
