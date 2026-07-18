import { Link } from 'react-router-dom';

// Shared pill button. Variants/sizes match what's already used across the app;
// pass `className` for one-off tweaks (extra margin, a border, etc.).
//
// Rendering: pass `to` to render a react-router <Link> styled as a button
// (used by the marketing CTAs); otherwise a <button> (defaults to
// type="button" so it never submits a form by accident — set type="submit"
// explicitly when needed).
//
// Hover is a simple opacity dim (no ring / no color invert). font-weight lives
// in each VARIANT, not BASE, so the two never fight over CSS source order.

const BASE =
  'inline-flex items-center justify-center rounded-full transition-opacity duration-200 hover:opacity-80 disabled:cursor-not-allowed disabled:opacity-50';

const VARIANTS = {
  primary: 'bg-white font-semibold text-black',
  ghost: 'font-medium text-slate-300',
  danger: 'bg-red-500 font-semibold text-white',
  dangerOutline: 'border border-red-500/50 font-semibold text-red-400',
};

const SIZES = {
  sm: 'px-3 py-1 text-xs',
  md: 'px-4 py-2 text-sm',
  lg: 'px-6 py-3 text-sm',
};

export default function Button({
  variant = 'primary',
  size = 'md',
  type = 'button',
  className = '',
  to,
  ...props
}) {
  const classes = `${BASE} ${VARIANTS[variant]} ${SIZES[size]} ${className}`;
  if (to) {
    return <Link to={to} className={classes} {...props} />;
  }
  return <button type={type} className={classes} {...props} />;
}
