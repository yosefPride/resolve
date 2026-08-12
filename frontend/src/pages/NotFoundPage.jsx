import { useAuth } from '../hooks/useAuth';
import Button from '../components/ui/Button';
import AppLayout from '../components/layout/AppLayout';

function NotFoundMessage({ homeTo, homeLabel }) {
  return (
    <section className="mx-auto flex max-w-2xl flex-col items-center gap-4 px-4 py-20 text-center sm:px-6 lg:px-8">
      <p className="text-[7rem] font-medium text-red-700">404</p>
      <h1 className="text-2xl font-bold text-white">Page not found</h1>
      <p className="text-sm text-slate-400">
        That page doesn't exist, or it may have moved.
      </p>
      <Button to={homeTo} className="mt-2">
        {homeLabel}
      </Button>
    </section>
  );
}

// Sits outside both route groups (see App.jsx), so it renders its own chrome
// rather than inheriting a layout: signed-in visitors get the regular
// Sidebar/AppLayout frame with a way back to their dashboard, everyone else
// gets the bare public look with no header or footer.
export default function NotFoundPage() {
  const { status } = useAuth();
  const isAuthed = status === 'authenticated';

  if (isAuthed) {
    return (
      <AppLayout>
        <NotFoundMessage homeTo="/dashboard" homeLabel="Go to dashboard" />
      </AppLayout>
    );
  }

  return (
    <div className="flex min-h-screen flex-col bg-black">
      <NotFoundMessage homeTo="/" homeLabel="Back home" />
    </div>
  );
}
