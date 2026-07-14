import api from '../lib/axios';

export function register({ email, password, name }) {
  return api.post('/auth/register', { email, password, name }).then((res) => res.data);
}

export function login({ email, password }) {
  return api.post('/auth/login', { email, password }).then((res) => res.data);
}

export function logout() {
  return api.post('/auth/logout').then((res) => res.data);
}

export function refresh() {
  return api.post('/auth/refresh').then((res) => res.data);
}

export function me() {
  return api.get('/auth/me').then((res) => res.data);
}
