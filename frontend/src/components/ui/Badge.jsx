// Small status/label pill. `neutral` is the default gray pill (roles, action
// labels); `accent` is the sky-tinted variant (e.g. the System Admin marker);
// `outline` is the muted bordered chip (e.g. the footer version tag). Sizes:
// `md` (default) and `sm` for tighter chips. Pass `className` for one-offs.

const BASE = 'inline-flex items-center rounded-full font-medium';

const VARIANTS = {
  neutral: 'bg-white/10 text-slate-300',
  accent: 'border border-sky-400/30 bg-sky-500/10 text-sky-300',
  outline: 'border border-white/10 text-slate-500',
};

const SIZES = {
  sm: 'px-2 py-0.5 text-xs',
  md: 'px-3 py-1 text-xs',
};

export default function Badge({ variant = 'neutral', size = 'md', className = '', ...props }) {
  return <span className={`${BASE} ${VARIANTS[variant]} ${SIZES[size]} ${className}`} {...props} />;
}
