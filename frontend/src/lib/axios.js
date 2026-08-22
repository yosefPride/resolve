import axios from 'axios';

const api = axios.create({
  baseURL: import.meta.env.VITE_API_URL,
  withCredentials: true,
  // Without this, a silently dropped network (Wi-Fi off, blackholed packets)
  // leaves a request pending indefinitely rather than rejecting — callers hang
  // on "Loading…" forever. A timeout rejects with no `error.response`, the same
  // response-less shape every call site already handles (see utils/errors.js).
  timeout: 15000,
});

let accessToken = null;

export function setAccessToken(token) {
  accessToken = token;
}

api.interceptors.request.use((config) => {
  if (accessToken) {
    config.headers.Authorization = `Bearer ${accessToken}`;
  }
  return config;
});

let unauthorizedHandler = null;

// Called by AuthContext once, at mount, so this module (which has no React/
// router access of its own) can hand control back to it when a refresh
// ultimately fails mid-session.
export function setUnauthorizedHandler(handler) {
  unauthorizedHandler = handler;
}

// Refresh tokens are single-use, so concurrent 401s must share one in-flight
// refresh rather than each firing their own — a second request would arrive
// with an already-revoked token and fail.
let refreshPromise = null;

function refreshAccessToken() {
  if (!refreshPromise) {
    refreshPromise = api
      .post('/auth/refresh')
      .then((data) => {
        setAccessToken(data.jwt);
        return data.jwt;
      })
      .finally(() => {
        refreshPromise = null;
      });
  }
  return refreshPromise;
}

const NO_RETRY_PATHS = ['/auth/login', '/auth/register', '/auth/refresh'];

// Successful responses resolve to the parsed body directly — every caller
// wants `res.data` and nothing else, so it's unwrapped once here instead of
// in a `.then((res) => res.data)` on all ~50 service calls. Errors are not
// unwrapped: they reject with the full axios error, whose `err.response` is
// what utils/errors.js and the field-error call sites read.
api.interceptors.response.use(
  (response) => response.data,
  async (error) => {
    const originalRequest = error.config;
    const shouldAttemptRefresh =
      error.response?.status === 401 &&
      originalRequest &&
      !originalRequest._retry &&
      !NO_RETRY_PATHS.includes(originalRequest.url);

    if (!shouldAttemptRefresh) {
      return Promise.reject(error);
    }

    originalRequest._retry = true;

    try {
        const jwt = await refreshAccessToken();
        originalRequest.headers.Authorization = `Bearer ${jwt}`;
        return api(originalRequest);
    } catch (refreshError) {
        setAccessToken(null);
        unauthorizedHandler?.();
        return Promise.reject(refreshError);
    }
  },
);

export default api;
