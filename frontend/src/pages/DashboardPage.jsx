import { useAuth } from '../hooks/useAuth';
import DashboardStats from '../features/dashboard/DashboardStats';

export default function DashboardPage() {
  const { user } = useAuth();

  return (
    <section className="flex flex-col gap-6">
      <h1 className="text-2xl font-bold text-white">Welcome, {user?.name}</h1>
      <DashboardStats />
    </section>
  );
}
