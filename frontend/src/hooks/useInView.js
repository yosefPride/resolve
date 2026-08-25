import { useEffect, useRef, useState } from 'react';

// Reports when the element first scrolls into view, then stops observing —
// used by the landing page's showcase tiles so their looping animations only
// run once the card is actually on screen, not the whole time the tab is open.
//
// Returns [ref, inView]; attach the ref to the element you want watched.
export default function useInView() {
  const ref = useRef(null);
  const [inView, setInView] = useState(false);

  useEffect(() => {
    const element = ref.current;
    if (!element) return undefined;

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (!entry.isIntersecting) return;
        setInView(true);
        observer.disconnect();
      },
      // Wait until the tile is a little way up the viewport, so the animation
      // doesn't play out while the card is still clipped by the fold.
      { rootMargin: '0px 0px -15% 0px' }
    );

    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  return [ref, inView];
}
