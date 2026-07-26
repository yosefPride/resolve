import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import App from './App.jsx'
import { AuthProvider } from './lib/AuthContext.jsx'
import './main.css'

const queryClient = new QueryClient({
  // retry: 1 so a failed request surfaces the error state quickly (RQ defaults
  // to 3 retries with backoff, which is slow to fail during manual testing).
  defaultOptions: { queries: { retry: 1 } },
})

createRoot(document.getElementById('root')).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <AuthProvider>
          <App />
        </AuthProvider>
      </BrowserRouter>
    </QueryClientProvider>
  </StrictMode>,
)
