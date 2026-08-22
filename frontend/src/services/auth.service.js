import api from '../lib/axios';

export function register({ email, password, name }) {
  return api.post('/auth/register', { email, password, name });
}

export function login({ email, password }) {
  return api.post('/auth/login', { email, password });
}

export function logout() {
  return api.post('/auth/logout');
}

export function refresh() {
  return api.post('/auth/refresh');
}

export function me() {
  return api.get('/auth/me');
}

// current_password is only required by the backend when the email changes;
// callers omit it for a name-only update.
export function updateProfile({ name, email, current_password }) {
  return api.patch('/auth/me', { name, email, current_password });
}

// Returns 200 with no body. On success the backend revokes every other
// session's refresh token; this one (its refresh cookie) stays valid.
export function changePassword({ current_password, new_password }) {
  return api.post('/auth/me/password', { current_password, new_password });
}
