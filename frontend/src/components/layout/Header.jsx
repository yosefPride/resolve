import { Link } from 'react-router-dom';
import Button from '../ui/Button';
import logo from '../../assets/brand-logo.svg';

// Always the logged-out face: signed-in pages live under AppLayout/Sidebar,
// which owns their nav and account menu instead.
//
// The bar floats: `fixed` takes it out of flow so the hero starts at the top of
// the viewport and the beams run behind it. The full-width strip is
// pointer-events-none so only the pill itself is clickable — otherwise the
// transparent band would swallow clicks across the whole page width.
//
// The wrapper repeats the landing sections' `max-w-7xl px-4 sm:px-6 lg:px-8`
// so the pill's edges land exactly on the column every section below shares.
export default function Header() {
  return (
    <header className="pointer-events-none fixed inset-x-0 top-4 z-50">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="pointer-events-auto flex h-14 items-center justify-between rounded-lg border border-white/10 bg-white/5 px-4 backdrop-blur-xl sm:px-6">
          <Link to="/" className="group flex items-center">
            <img
              src={logo}
              alt="Resolve"
              className="h-6 w-auto object-contain transition-all duration-200 group-hover:drop-shadow-[0_0_10px_rgba(56,189,248,0.8)]"
            />
          </Link>

          <div className="flex items-center gap-2">
            <Button to="/login" variant="ghost">
              Log in
            </Button>
            <Button to="/register">
              Sign up
            </Button>
          </div>
        </div>
      </div>
    </header>
  );
}
