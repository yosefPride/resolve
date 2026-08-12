import { Link } from 'react-router-dom';
import Button from '../ui/Button';
import logo from '../../assets/brand-logo.svg';

// Always the logged-out face: signed-in pages live under AppLayout/Sidebar,
// which owns their nav and account menu instead.
export default function Header() {
  return (
    <header className="sticky top-0 z-50 bg-black/70 backdrop-blur-md">
      <div className="mx-auto flex h-20 max-w-7xl items-center justify-between px-4 sm:px-6 lg:px-8">
        <Link to="/" className="group flex items-center">
          <img
            src={logo}
            alt="Resolve"
            className="h-6 w-auto object-contain transition-all duration-200 group-hover:drop-shadow-[0_0_10px_rgba(56,189,248,0.8)]"
          />
        </Link>

        <div className="flex items-center gap-3">
          <Button to="/login" variant="ghost">
            Log in
          </Button>
          <Button to="/register">
            Sign up
          </Button>
        </div>
      </div>

      <div className="h-px bg-white/10" />
    </header>
  );
}
