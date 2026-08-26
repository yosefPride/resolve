import logo from '../../assets/brand-logo.svg';
import Badge from '../ui/Badge';
import HexagonBackground from '../marketing/background/HexagonBackground';

const FOOTER_LINKS = [];

// The honeycomb reacts to the cursor, so the content layer above it is
// pointer-events-none and only the actual links opt back in — otherwise the
// centred column would block hover across most of the footer's width.
export default function Footer() {
  const year = new Date().getFullYear();

  return (
    <footer className="relative isolate overflow-hidden border-t border-white/10 bg-black">
      <HexagonBackground className="absolute inset-0" hexagonSize={60} hexagonMargin={4} />

      {/* Fades the lattice out toward both edges so it emerges from the page
          rather than starting/ending abruptly at the borders. */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-0 top-0 h-32 bg-linear-to-b from-black to-transparent"
      />
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-0 bottom-0 h-32 bg-linear-to-t from-black to-transparent"
      />

      <div className="pointer-events-none relative mx-auto flex max-w-7xl flex-col items-center gap-3 px-4 py-24 sm:flex-row sm:justify-between sm:px-6 lg:px-8">
        <div className="flex items-center gap-2">
          <img src={logo} alt="" className="h-3 w-auto object-contain opacity-50" />
          <span className="text-xs text-slate-500">© {year} </span>
        </div>

        <div className="flex items-center gap-3">
          {FOOTER_LINKS.map((link) => (
            <a
              key={link.to}
              href={link.to}
              className="pointer-events-auto rounded-full px-2 py-0.5 text-xs text-slate-500 transition-colors hover:bg-white/10 hover:text-white"
            >
              {link.label}
            </a>
          ))}
          <Badge variant="outline" size="sm">
            v0.1.0
          </Badge>
        </div>
      </div>
    </footer>
  );
}
