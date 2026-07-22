import { useState } from 'react';
import { useAuth } from '../../hooks/useAuth';
import { updateProfile } from '../../services/auth.service';
import { errorMessage } from '../../utils/errors';
import Button from '../../components/ui/Button';
import Input from '../../components/ui/Input';

export default function ProfileForm() {
  const { user, updateUser } = useAuth();
  const [form, setForm] = useState({ name: user.name, email: user.email, currentPassword: '' });
  const [error, setError] = useState('');
  const [emailError, setEmailError] = useState('');
  const [success, setSuccess] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const nameChanged = form.name.trim() !== user.name;
  const emailChanged = form.email.trim() !== user.email;
  const isDirty = nameChanged || emailChanged;

  function handleChange(event) {
    const { name, value } = event.target;
    setForm((prev) => ({ ...prev, [name]: value }));
    setError('');
    setEmailError('');
    setSuccess('');
  }

  async function handleSubmit(event) {
    event.preventDefault();
    setError('');
    setEmailError('');
    setSuccess('');
    setIsSubmitting(true);
    try {
      // Send only what changed; current_password is required by the backend
      // only when the email changes (it's the login identity).
      const updated = await updateProfile({
        name: nameChanged ? form.name.trim() : undefined,
        email: emailChanged ? form.email.trim() : undefined,
        current_password: emailChanged ? form.currentPassword : undefined,
      });
      updateUser(updated);
      setForm({ name: updated.name, email: updated.email, currentPassword: '' });
      setSuccess('Profile updated.');
    } catch (err) {
      // A taken email is a field problem — show it under the Email input;
      // everything else (wrong password, validation) stays form-level.
      if (err.response?.data?.error?.code === 'duplicate_email') {
        setEmailError('Another account is already using this email address.');
      } else {
        setError(errorMessage(err, 'Failed to update profile.'));
      }
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <div className="rounded-lg border border-white/10 bg-white/5 p-6">
      <h2 className="text-lg font-semibold text-white">Profile</h2>
      <p className="mt-1 text-sm text-slate-400">Update your name and email address.</p>

      <form onSubmit={handleSubmit} className="mt-4 flex flex-col gap-4">
        <label className="flex flex-col gap-1 text-sm text-slate-300">
          Name
          <Input
            type="text"
            name="name"
            value={form.name}
            onChange={handleChange}
            required
          />
        </label>

        <label className="flex flex-col gap-1 text-sm text-slate-300">
          Email
          <Input
            type="email"
            name="email"
            value={form.email}
            onChange={handleChange}
            required
          />
          {emailError && <span className="text-sm text-red-500">{emailError}</span>}
        </label>

        {emailChanged && (
          <label className="flex flex-col gap-1 text-sm text-slate-300">
            Current password
            <Input
              type="password"
              name="currentPassword"
              value={form.currentPassword}
              onChange={handleChange}
              required
              autoComplete="current-password"
            />
            <span className="text-xs text-slate-400">Required to change your email.</span>
          </label>
        )}

        {error && <p className="text-sm text-red-500">{error}</p>}
        {success && <p className="text-sm text-green-400">{success}</p>}

        <Button type="submit" disabled={isSubmitting || !isDirty} className="mt-2 self-start">
          {isSubmitting ? 'Saving…' : 'Save changes'}
        </Button>
      </form>
    </div>
  );
}
