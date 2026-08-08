import LinksSection from './LinksSection';
import ReferencesSection from './ReferencesSection';

// The two sub-sections behind the Links tab, per the locked scope: relation
// links between tickets, and external URL references — distinct backend
// resources (ticket_links vs ticket_references), shown together here since
// both answer "what else does this issue connect to".
export default function LinksPanel({ groupId, ticketId, isAdmin }) {
  return (
    <div className="flex flex-col gap-6">
      <LinksSection groupId={groupId} ticketId={ticketId} isAdmin={isAdmin} />
      <div className="border-t border-white/10" />
      <ReferencesSection groupId={groupId} ticketId={ticketId} isAdmin={isAdmin} />
    </div>
  );
}
