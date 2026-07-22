import { useAuth } from '../hooks/useAuth';
import Button from '../components/ui/Button';

export default function NotFoundPage() {
  const { status } = useAuth();
  // While the boot refresh is still resolving we don't know yet, so point at
  // the landing page — it's reachable either way.
  const isAuthed = status === 'authenticated';

  return (
    <section className="mx-auto flex max-w-2xl flex-col items-center gap-4 px-4 py-20 text-center sm:px-6 lg:px-8">
      <p className="text-sm font-medium text-sky-400">404</p>
      <h1 className="text-2xl font-bold text-white">Page not found</h1>
      <p className="text-sm text-slate-400">
        That page doesn't exist, or it may have moved.
      </p>
      <Button to={isAuthed ? '/dashboard' : '/'} className="mt-2">
        {isAuthed ? 'Back to dashboard' : 'Back home'}
      </Button>
    </section>
  );
}
