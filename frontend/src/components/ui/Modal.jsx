import * as Dialog from '@radix-ui/react-dialog';

// Thin wrapper over Radix Dialog that keeps the app's existing Modal API
// ({ isOpen, onClose, title, children }). Radix handles Escape, outside-click,
// focus trapping, scroll lock, and aria-modal. A Dialog.Title is always
// rendered (visually hidden when no `title` is given) since Radix requires one
// for accessibility.
export default function Modal({ isOpen, onClose, title, children }) {
  return (
    <Dialog.Root open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm" />
        <Dialog.Content
          aria-describedby={undefined}
          className="fixed left-1/2 top-1/2 z-50 w-[calc(100%-2rem)] max-w-md -translate-x-1/2 -translate-y-1/2 rounded-lg border border-white/10 bg-neutral-950 p-6 shadow-2xl shadow-black/50"
        >
          <Dialog.Title className={title ? 'mb-4 text-lg font-semibold text-white' : 'sr-only'}>
            {title || 'Dialog'}
          </Dialog.Title>
          {children}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
