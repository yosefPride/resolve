import { useAuth } from '../hooks/useAuth';
import DashboardStats from '../features/dashboard/DashboardStats';

export default function DashboardPage() {
  const { user } = useAuth();

  return (
    <section className="mx-auto flex max-w-4xl flex-col gap-6 px-4 py-20 sm:px-6 lg:px-8">
      <h1 className="text-2xl font-bold text-white">Welcome, {user?.name}</h1>
      <DashboardStats />
    </section>
  );
}
