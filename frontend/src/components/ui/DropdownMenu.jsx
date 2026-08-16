import * as RadixDropdownMenu from '@radix-ui/react-dropdown-menu';

// Thin wrapper over Radix DropdownMenu that carries the app's shared look
// (solid gray panel). Rendered through a Portal, so unlike the issue page's
// bg-white/5 tabs — which always sit on the same dark panel — this can float
// over anything; a translucent tint would show whatever's behind it through,
// so the panel needs its own opaque color instead. `trigger` is rendered as
// the actual trigger element via asChild, so callers keep full control over
// its markup (icon, aria-label, disabled, etc.) — this component only owns
// the popover shell. `open`/`onOpenChange` are optional: pass both for a
// caller that needs to close the panel itself (e.g. after picking one item
// from a non-Item grid of buttons); omit both and Radix manages open state
// on its own, as every existing caller does.
export default function DropdownMenu({
  trigger,
  children,
  align = 'end',
  sideOffset = 8,
  width = 'w-40',
  contentClassName = '',
  open,
  onOpenChange,
}) {
  return (
    <RadixDropdownMenu.Root open={open} onOpenChange={onOpenChange}>
      <RadixDropdownMenu.Trigger asChild>{trigger}</RadixDropdownMenu.Trigger>
      <RadixDropdownMenu.Portal>
        <RadixDropdownMenu.Content
          align={align}
          sideOffset={sideOffset}
          className={`z-50 ${width} rounded-lg border border-white/10 bg-neutral-800 py-1 shadow-2xl shadow-black/50 ${contentClassName}`}
        >
          {children}
        </RadixDropdownMenu.Content>
      </RadixDropdownMenu.Portal>
    </RadixDropdownMenu.Root>
  );
}

// `width` is its own prop (not folded into contentClassName) so callers that
// need a different panel width swap the whole utility instead of stacking a
// second `w-*` class next to the default — Tailwind resolves conflicting
// utilities by CSS source order, not by position in the className string, so
// two `w-*` classes together would be a silent toss-up.
const ITEM_VARIANTS = {
  default: 'text-slate-300',
  danger: 'text-red-400',
  // No color baked in — for items whose color is conditional/caller-owned
  // (e.g. active vs inactive), same reasoning as `width` above.
  plain: '',
};

export function DropdownMenuItem({ variant = 'default', className = '', ...props }) {
  return (
    <RadixDropdownMenu.Item
      className={`cursor-pointer px-4 py-2 text-sm outline-none transition-colors data-highlighted:bg-white/10 ${ITEM_VARIANTS[variant]} ${className}`}
      {...props}
    />
  );
}
