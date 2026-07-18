// Small status/label pill. `neutral` is the default gray pill (roles, action
// labels); `accent` is the sky-tinted variant (e.g. the System Admin marker).
// Pass `className` for one-off tweaks.

const BASE = 'inline-flex items-center rounded-full px-3 py-1 text-xs font-medium';

const VARIANTS = {
  neutral: 'bg-white/10 text-slate-300',
  accent: 'border border-sky-400/30 bg-sky-500/10 text-sky-300',
};

export default function Badge({ variant = 'neutral', className = '', ...props }) {
  return <span className={`${BASE} ${VARIANTS[variant]} ${className}`} {...props} />;
}
