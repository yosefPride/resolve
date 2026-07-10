import { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import * as authService from '../services/auth.service';
import Spinner from '../components/ui/Spinner';
import { setAccessToken, setUnauthorizedHandler } from './axios';
import { AuthContext } from './authContext';

// Module-scoped (not component state) so React StrictMode's dev-only double
// effect-invocation reuses the same in-flight call instead of firing a second
// /auth/refresh — the refresh token is single-use, so a genuine duplicate
// request would 401 and could flip a valid session to logged-out.
let bootstrapPromise = null;

function bootstrapSession() {
  if (!bootstrapPromise) {
    bootstrapPromise = (async () => {
      const { jwt } = await authService.refresh();
      setAccessToken(jwt);
      return authService.me();
    })();
  }
  return bootstrapPromise;
}

export function AuthProvider({ children }) {
  const [user, setUser] = useState(null);
  const [status, setStatus] = useState('loading');
  const navigate = useNavigate();

  useEffect(() => {
    setUnauthorizedHandler(() => {
      setUser(null);
      setStatus('unauthenticated');
      navigate('/login');
    });
  }, [navigate]);

  useEffect(() => {
    let cancelled = false;

    bootstrapSession()
      .then((user) => {
        if (cancelled) return;
        setUser(user);
        setStatus('authenticated');
      })
      .catch(() => {
        if (cancelled) return;
        setAccessToken(null);
        setStatus('unauthenticated');
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const register = useCallback(async (input) => {
    const { user, jwt } = await authService.register(input);
    setAccessToken(jwt);
    setUser(user);
    setStatus('authenticated');
  }, []);

  const login = useCallback(async (input) => {
    const { user, jwt } = await authService.login(input);
    setAccessToken(jwt);
    setUser(user);
    setStatus('authenticated');
  }, []);

  const logout = useCallback(async () => {
    await authService.logout();
    setAccessToken(null);
    setUser(null);
    setStatus('unauthenticated');
    navigate('/login');
  }, [navigate]);

  if (status === 'loading') {
    return <Spinner />;
  }

  return (
    <AuthContext.Provider value={{ user, status, login, register, logout }}>
      {children}
    </AuthContext.Provider>
  );
}
