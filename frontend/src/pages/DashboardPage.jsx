import DashboardHeader from '../features/dashboard/DashboardHeader';
import DashboardStats from '../features/dashboard/DashboardStats';
import GroupsList from '../features/dashboard/GroupsList';
import RecentTickets from '../features/dashboard/RecentTickets';

export default function DashboardPage() {
  return (
    <section className="flex flex-col gap-6">
      <DashboardHeader />
      <DashboardStats />
      <div className="grid grid-cols-1 gap-6 border-t border-white/10 pt-6 lg:grid-cols-[7fr_3fr]">
        <RecentTickets />
        <GroupsList />
      </div>
    </section>
  );
}
