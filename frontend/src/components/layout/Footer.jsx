import logo from '../../assets/logo.png';

const FOOTER_LINKS = [];

export default function Footer() {
  const year = new Date().getFullYear();

  return (
    <footer className="border-t border-white/10 bg-black">
      <div className="mx-auto flex max-w-7xl flex-col items-center gap-3 px-4 py-6 sm:flex-row sm:justify-between sm:px-6 lg:px-8">
        <div className="flex items-center gap-2">
          <img src={logo} alt="" className="h-10 w-auto object-contain opacity-50" />
          <span className="text-xs text-slate-500">© {year} Resolve</span>
        </div>

        <div className="flex items-center gap-3">
          {FOOTER_LINKS.map((link) => (
            <a
              key={link.to}
              href={link.to}
              className="rounded-full px-2 py-0.5 text-xs text-slate-500 transition-colors hover:bg-white/10 hover:text-white"
            >
              {link.label}
            </a>
          ))}
          <span className="rounded-full border border-white/10 px-2 py-0.5 text-xs text-slate-500">
            v0.1.0
          </span>
        </div>
      </div>
    </footer>
  );
}
