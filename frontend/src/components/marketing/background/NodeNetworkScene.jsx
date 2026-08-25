import { useEffect, useState } from 'react';
import { Canvas } from '@react-three/fiber';
import NodeNetwork from './NodeNetwork';

// Camera/canvas shell for <NodeNetwork>. Kept separate so Hero can lazy-load
// this file — importing it pulls in three.js (~150KB gzipped), which has no
// business in the landing page's initial bundle.
//
// The canvas is transparent (no `background` style, alpha on by default) so the
// hero's white glow keeps showing through behind the plexus.
export default function NodeNetworkScene(props) {
  const [animate, setAnimate] = useState(true);

  useEffect(() => {
    const query = window.matchMedia('(prefers-reduced-motion: reduce)');
    const sync = () => setAnimate(!query.matches);
    sync();
    query.addEventListener('change', sync);
    return () => query.removeEventListener('change', sync);
  }, []);

  return (
    <Canvas camera={{ position: [0, 0, 9], fov: 60 }} dpr={[1, 2]}>
      <NodeNetwork animate={animate} {...props} />
    </Canvas>
  );
}
