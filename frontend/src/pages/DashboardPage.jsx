import { useAuth } from '../hooks/useAuth';
import Button from '../components/ui/Button';

export default function DashboardPage() {
  const { user, logout } = useAuth();

  return (
    <section className="mx-auto flex max-w-md flex-col items-center gap-6 px-4 py-20 text-center sm:px-6 lg:px-8">
      <h1 className="text-2xl font-bold text-white">Welcome, {user?.name}</h1>
      <Button onClick={() => logout()}>Log out</Button>
    </section>
  );
}
