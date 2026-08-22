// Labeled form field: the standard label styling around a control, with an
// optional red error line after it. Extra content (hints, helper text) goes
// in children alongside the control. The audit-log filters and
// DeleteUserModal keep their own smaller text-xs labels.
export default function Field({ label, error, className = '', children }) {
  return (
    <label className={`flex flex-col gap-1 text-sm text-slate-300 ${className}`}>
      {label}
      {children}
      {error && <span className="text-sm text-red-500">{error}</span>}
    </label>
  );
}
