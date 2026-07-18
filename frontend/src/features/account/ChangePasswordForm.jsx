import { useState } from 'react';
import { changePassword } from '../../services/auth.service';
import { errorMessage } from '../../utils/errors';
import Button from '../../components/ui/Button';

const MIN_PASSWORD_LENGTH = 8;

const INPUT_CLASS =
  'rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50';

const EMPTY_FORM = { currentPassword: '', newPassword: '', confirmPassword: '' };

export default function ChangePasswordForm() {
  const [form, setForm] = useState(EMPTY_FORM);
  const [error, setError] = useState('');
  const [fieldErrors, setFieldErrors] = useState({});
  const [success, setSuccess] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  function handleChange(event) {
    const { name, value } = event.target;
    setForm((prev) => ({ ...prev, [name]: value }));
    setError('');
    setFieldErrors({});
    setSuccess('');
  }

  async function handleSubmit(event) {
    event.preventDefault();
    setError('');
    setSuccess('');

    // Client-side checks mirror the backend's min length and catch the
    // confirm mismatch before a round-trip.
    const errors = {};
    if (form.newPassword.length < MIN_PASSWORD_LENGTH) {
      errors.newPassword = `Password must be at least ${MIN_PASSWORD_LENGTH} characters.`;
    }
    if (form.confirmPassword !== form.newPassword) {
      errors.confirmPassword = 'Passwords do not match.';
    }
    if (Object.keys(errors).length > 0) {
      setFieldErrors(errors);
      return;
    }
    setFieldErrors({});

    setIsSubmitting(true);
    try {
      await changePassword({
        current_password: form.currentPassword,
        new_password: form.newPassword,
      });
      setForm(EMPTY_FORM);
      setSuccess('Password changed. Other devices have been signed out.');
    } catch (err) {
      // A wrong current password is a field problem — show it under that
      // input rather than as the backend's login-oriented message.
      if (err.response?.data?.error?.code === 'invalid_credentials') {
        setFieldErrors({ currentPassword: 'Current password is incorrect.' });
      } else {
        setError(errorMessage(err, 'Failed to change password.'));
      }
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <div className="rounded-lg border border-white/10 bg-white/5 p-6">
      <h2 className="text-lg font-semibold text-white">Password</h2>
      <p className="mt-1 text-sm text-slate-400">
        Changing your password signs you out on all other devices.
      </p>

      <form onSubmit={handleSubmit} className="mt-4 flex flex-col gap-4">
        <label className="flex flex-col gap-1 text-sm text-slate-300">
          Current password
          <input
            type="password"
            name="currentPassword"
            value={form.currentPassword}
            onChange={handleChange}
            required
            autoComplete="current-password"
            className={INPUT_CLASS}
          />
          {fieldErrors.currentPassword && (
            <span className="text-sm text-red-500">{fieldErrors.currentPassword}</span>
          )}
        </label>

        <label className="flex flex-col gap-1 text-sm text-slate-300">
          New password
          <input
            type="password"
            name="newPassword"
            value={form.newPassword}
            onChange={handleChange}
            required
            autoComplete="new-password"
            className={INPUT_CLASS}
          />
          {fieldErrors.newPassword && (
            <span className="text-sm text-red-500">{fieldErrors.newPassword}</span>
          )}
        </label>

        <label className="flex flex-col gap-1 text-sm text-slate-300">
          Confirm new password
          <input
            type="password"
            name="confirmPassword"
            value={form.confirmPassword}
            onChange={handleChange}
            required
            autoComplete="new-password"
            className={INPUT_CLASS}
          />
          {fieldErrors.confirmPassword && (
            <span className="text-sm text-red-500">{fieldErrors.confirmPassword}</span>
          )}
        </label>

        {error && <p className="text-sm text-red-500">{error}</p>}
        {success && <p className="text-sm text-green-400">{success}</p>}

        <Button type="submit" disabled={isSubmitting} className="mt-2 self-start">
          {isSubmitting ? 'Changing…' : 'Change password'}
        </Button>
      </form>
    </div>
  );
}
