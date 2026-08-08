import DashboardHeader from '../features/dashboard/DashboardHeader';
import DashboardStats from '../features/dashboard/DashboardStats';
import RecentTickets from '../features/dashboard/RecentTickets';

export default function DashboardPage() {
  return (
    <section className="flex flex-col gap-6">
      <DashboardHeader />
      <DashboardStats />
      <RecentTickets />
    </section>
  );
}
