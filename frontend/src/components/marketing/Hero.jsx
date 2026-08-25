import { lazy, Suspense } from 'react';
import Button from '../ui/Button';
// import ProductDemo from './demo/ProductDemo';

// Lazy so three.js stays out of the landing page's initial bundle — the hero
// renders text-first and the plexus fades in when its chunk lands.
const NodeNetworkScene = lazy(() => import('./background/NodeNetworkScene'));

// `isolate` on the section keeps the -z-10 background layer in a stacking
// context of its own. Without it it belongs to the root context, where it
// paints beneath the layout's bg-black and disappears.
export default function Hero() {
  return (
    <section className="relative isolate h-[82vh] overflow-hidden">
      <div className="relative mx-auto max-w-7xl px-4 pt-20 pb-24 text-left sm:px-6 sm:pt-28 lg:px-8">
        {/* The flat white blur glow these beams replaced, kept in case you
            want it back.
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-x-0 top-20 -z-10 h-225 w-screen bg-linear-to-b from-white/10 via-white/20 to-white/60 blur-[180px]"
        />
        */}

        {/* Light beams: dim at the top, widening and brightening toward the
            bottom. Each beam is a trapezoid (clip-path, narrow top edge → wide
            bottom edge) filled with a downward gradient. The blur lives on this
            wrapper rather than on the beams because clip-path is applied after
            filter — blurring a beam directly would still leave the clip's hard
            edges. w-screen plus the centering translate breaks it out of the
            max-w-7xl container so the beams reach both page edges. */}
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-y-0 left-1/2 -z-10 w-screen -translate-x-1/2 overflow-hidden blur-[90px]"
        >
          <div className="absolute inset-y-0 left-1/2 w-[65%] -translate-x-1/2 bg-linear-to-b from-transparent via-white/4 to-white/12 [clip-path:polygon(44%_0%,56%_0%,100%_100%,0%_100%)]" />
          <div className="absolute inset-y-0 left-1/2 w-[95%] -translate-x-[70%] bg-linear-to-b from-transparent via-white/2 to-white/5 [clip-path:polygon(46%_0%,52%_0%,88%_100%,18%_100%)]" />
          <div className="absolute inset-y-0 left-1/2 w-[95%] -translate-x-[30%] bg-linear-to-b from-transparent via-white/2 to-white/5 [clip-path:polygon(48%_0%,54%_0%,82%_100%,12%_100%)]" />
        </div>

        {/* Copy and animation share one row from `lg` up. Below that the grid
            collapses to a single column and the plexus moves above the copy
            (order-first), centered and capped in width, with less height — a
            full-size canvas on a phone is both a scroll hazard and a battery
            drain. */}
        <div className="grid items-center gap-12 lg:grid-cols-2 lg:gap-8">
          <div>
            <h1 className="mt-6 text-4xl font-bold tracking-tight text-white sm:text-5xl lg:text-6xl">
              Track. Discuss. Resolve.
            </h1>

            <p className="mt-5 max-w-xl text-base text-slate-400 sm:text-lg">
              Resolve combines issue tracking and team management — powered by AI where it counts.
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

          {/* Product demo temporarily swapped out for the Facet node-network
              animation. Comment the <Suspense> block out and uncomment this to
              put the real preview back — note the demo needs the full width,
              so restoring it means undoing this two-column grid too.
          <ProductDemo />
          */}
          <Suspense fallback={<div className="order-first mx-auto h-75 w-full max-w-sm lg:order-none lg:h-125 lg:max-w-none" />}>
            <div className="order-first mx-auto h-75 w-full max-w-sm lg:order-none lg:h-125 lg:max-w-none">
              <NodeNetworkScene color="#ffffff" nodeCount={190} maxDistance={1.8} speed={0.35} />
            </div>
          </Suspense>
        </div>
      </div>
    </section>
  );
}
