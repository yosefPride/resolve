import { Link, useParams, useSearchParams } from 'react-router-dom';
import { ArrowLeft } from 'lucide-react';
import { useTicket } from '../hooks/useTickets';
import { useGroup } from '../hooks/useGroup';
import { useAuth } from '../hooks/useAuth';
import { isGroupAdmin } from '../utils/roles';
import TicketDetail from '../features/tickets/TicketDetail';
import TicketMeta from '../features/tickets/TicketMeta';

// The group is carried as ?group=<id> (see TicketsPage), not a route param —
// same "no active group in state" rule, just read from the URL here too.
export default function TicketDetailPage() {
  const { ticketId } = useParams();
  const [searchParams] = useSearchParams();
  const groupId = searchParams.get('group') || '';
  const { user } = useAuth();

  const { data: ticket, status: ticketStatus } = useTicket(groupId, ticketId);
  const { group, members, status: groupStatus } = useGroup(groupId);

  if (!groupId) {
    return (
      <section className="mx-auto max-w-2xl px-4 py-20 sm:px-6 lg:px-8">
        <p className="text-sm text-red-500">
          Missing team context. Open this issue from the Issues list instead.
        </p>
        <Link
          to="/tickets"
          className="mt-4 inline-flex items-center gap-1 text-sm text-slate-300 hover:text-white"
        >
          <ArrowLeft className="h-4 w-4" /> Back to Issues
        </Link>
      </section>
    );
  }

  if (ticketStatus === 'pending' || groupStatus === 'pending') {
    return (
      <section className="mx-auto max-w-2xl px-4 py-20 sm:px-6 lg:px-8">
        <p className="text-sm text-slate-400">Loading…</p>
      </section>
    );
  }

  if (ticketStatus === 'error' || groupStatus === 'error') {
    return (
      <section className="mx-auto max-w-2xl px-4 py-20 sm:px-6 lg:px-8">
        <p className="text-sm text-red-500">
          Couldn't load this issue. It may not exist, or you may not have access.
        </p>
        <Link
          to={`/tickets?group=${groupId}`}
          className="mt-4 inline-flex items-center gap-1 text-sm text-slate-300 hover:text-white"
        >
          <ArrowLeft className="h-4 w-4" /> Back to Issues
        </Link>
      </section>
    );
  }

  const myRole = members.find((member) => member.user_id === user.id)?.role;

  return (
    <section className="mx-auto flex max-w-6xl flex-col gap-6 px-4 py-20 sm:px-6 lg:px-8">
      <Link
        to={`/tickets?group=${groupId}`}
        className="inline-flex items-center gap-1 text-sm text-slate-400 hover:text-white"
      >
        <ArrowLeft className="h-4 w-4" /> Back to Issues
      </Link>
      <div className="grid items-start gap-6 lg:grid-cols-[minmax(0,1fr)_320px]">
        <TicketDetail
          ticket={ticket}
          teamName={group?.name}
          groupId={groupId}
          isAdmin={isGroupAdmin(myRole)}
        />
        <aside className="flex flex-col gap-6">
          <TicketMeta ticket={ticket} />
          {/* ai/ is a separate, not-yet-built feature (docs/frontend.md lists it
              as part of this page) — the placeholder holds its rail slot. */}
          <div className="rounded-xl border border-white/10 bg-white/5 p-6">
            <h2 className="text-sm font-semibold text-slate-400">AI</h2>
            <p className="mt-2 text-sm text-slate-500">Coming soon.</p>
          </div>
        </aside>
      </div>
    </section>
  );
}
