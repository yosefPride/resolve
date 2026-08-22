import { useState } from 'react';
import { errorMessage } from '../utils/errors';

// Shared submit choreography for simple forms: clear the error, run the
// action, map a failure through errorMessage. `submit` goes straight onto a
// form's onSubmit. Forms with field-level errors or special-cased status
// codes (ProfileForm, ChangePasswordForm, CommentForm) keep their own
// handlers rather than this hook growing options for them; `setError` is
// exposed for callers with a second error source (e.g. a lookup + mutation
// pair sharing one error line).
export function useSubmit(action, fallback) {
  const [error, setError] = useState('');
  const [isPending, setIsPending] = useState(false);

  async function submit(event) {
    event?.preventDefault();
    setError('');
    setIsPending(true);
    try {
      await action();
    } catch (err) {
      setError(errorMessage(err, fallback));
    } finally {
      setIsPending(false);
    }
  }

  return { error, setError, isPending, submit };
}
