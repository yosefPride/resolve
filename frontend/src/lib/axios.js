import axios from 'axios';

const api = axios.create({
  baseURL: import.meta.env.VITE_API_URL,
  withCredentials: true,
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
      .then((res) => {
        setAccessToken(res.data.jwt);
        return res.data.jwt;
      })
      .finally(() => {
        refreshPromise = null;
      });
  }
  return refreshPromise;
}

const NO_RETRY_PATHS = ['/auth/login', '/auth/register', '/auth/refresh'];

api.interceptors.response.use(
  (response) => response,
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
