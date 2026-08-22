import Modal from './Modal';
import Button from './Button';

// Shared body for the app's confirm dialogs: message, optional error line,
// then a Cancel + confirm button pair. Composes Modal rather than extending
// it, so Modal stays a thin general-purpose wrapper for form dialogs etc.
//
// Owns the behaviors the hand-rolled copies used to drift on: the dialog
// can't be closed (Cancel, Escape, overlay click) while the action is
// pending, and the confirm button swaps to pendingLabel while it runs.
// The message goes in `children` (JSX, so callers can bold names).
//
// `variant` is the confirm Button's variant — 'danger' (default) for
// destructive actions, 'primary' for non-destructive ones like promote.
export default function ConfirmModal({
  isOpen,
  onClose,
  title,
  confirmLabel,
  pendingLabel,
  variant = 'danger',
  isPending = false,
  error = '',
  onConfirm,
  children,
}) {
  function handleClose() {
    if (isPending) return;
    onClose();
  }

  return (
    <Modal isOpen={isOpen} onClose={handleClose} title={title}>
      <div className="text-sm text-slate-300">{children}</div>

      {error && <p className="mt-3 text-sm text-red-500">{error}</p>}

      <div className="mt-6 flex justify-end gap-3">
        <Button variant="ghost" disabled={isPending} onClick={handleClose}>
          Cancel
        </Button>
        <Button variant={variant} disabled={isPending} onClick={onConfirm}>
          {isPending ? pendingLabel : confirmLabel}
        </Button>
      </div>
    </Modal>
  );
}
