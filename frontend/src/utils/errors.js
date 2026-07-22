// Turns an axios error into a user-facing message.
// - No response (network down, request timeout, CORS): there's no server payload
//   to read, so return a network message. Timeouts arrive as code 'ECONNABORTED'.
// - Response present: use the API's { error: { message } } shape (see docs/api.md),
//   falling back to the caller's message if the body isn't shaped that way.
export function errorMessage(err, fallback = 'Something went wrong. Please try again.') {
  if (!err.response) {
    return 'Network error — check your connection and try again.';
  }
  return err.response.data?.error?.message || fallback;
}
