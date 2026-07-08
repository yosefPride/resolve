import { useAuth } from '../hooks/useAuth';

export default function DashboardPage() {
  const { user, logout } = useAuth();

  return (
    <section className="mx-auto flex max-w-md flex-col items-center gap-6 px-4 py-20 text-center sm:px-6 lg:px-8">
      <h1 className="text-2xl font-bold text-white">Welcome, {user?.name}</h1>
      <button
        type="button"
        onClick={() => logout()}
        className="rounded-full bg-white px-4 py-2 text-sm font-semibold text-black transition-all duration-200 hover:shadow-[0_0_15px_2px_rgba(255,255,255,0.5)]"
      >
        Log out
      </button>
    </section>
  );
}
