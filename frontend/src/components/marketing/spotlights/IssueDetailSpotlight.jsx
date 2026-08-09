import { useState } from 'react';
import { Activity, AlertTriangle, FileText, Link2, MessageSquare, Sparkles, Users } from 'lucide-react';
import Badge from '../../ui/Badge';
import Avatar from '../../ui/Avatar';
import DescriptionMarkdown from '../../../features/tickets/DescriptionMarkdown';
import { PRIORITY_VARIANT, STATUS_VARIANT, capitalize } from '../../../features/tickets/badgeVariants';
import FeatureSpotlight from './FeatureSpotlight';
import { DEMO_TEAM_NAME } from '../demo/demoIssues';
import {
  SPOTLIGHT_ACTIVITY,
  SPOTLIGHT_COMMENTS,
  SPOTLIGHT_LINK,
  SPOTLIGHT_TICKET,
} from '../demo/spotlightSeed';

// A static preview of the real ticket detail page (features/tickets/TicketDetail.jsx):
// same tab bar, same fixed-height panel treatment, same meta rail. Tab switching is
// real (local useState, no data fetching) so it doesn't read as a flat screenshot;
// nothing here calls the API or mutates anything, unlike the real TicketMeta (which
// this intentionally does not reuse — it fires a live update mutation and links to a
// real /groups/:id route, both wrong on a logged-out page).

const TABS = [
  { id: 'details', label: 'Details', icon: FileText },
  { id: 'comments', label: 'Comments', icon: MessageSquare },
  { id: 'links', label: 'Links', icon: Link2 },
  { id: 'activity', label: 'Activity', icon: Activity },
];

function TabButton({ isActive, onClick, icon: Icon, children }) {
  return (
    <button
      type="button"
      role="tab"
      aria-selected={isActive}
      onClick={onClick}
      className={`-mb-px inline-flex shrink-0 items-center gap-2 border-b-2 px-1 pb-3 text-sm font-medium transition-colors ${
        isActive ? 'border-sky-400 text-white' : 'border-transparent text-slate-400 hover:text-slate-200'
      }`}
    >
      <Icon className="h-4 w-4" />
      {children}
    </button>
  );
}

function ActivityText({ entry }) {
  switch (entry.event_type) {
    case 'ticket_created':
      return <>created this issue</>;
    case 'priority_changed':
      return (
        <>
          changed priority from{' '}
          <Badge size="sm" variant={PRIORITY_VARIANT[entry.old_value]}>
            {capitalize(entry.old_value)}
          </Badge>{' '}
          to{' '}
          <Badge size="sm" variant={PRIORITY_VARIANT[entry.new_value]}>
            {capitalize(entry.new_value)}
          </Badge>
        </>
      );
    case 'comment_added':
      return <>commented</>;
    default:
      return null;
  }
}

const ACTIVITY_ICON = {
  ticket_created: Sparkles,
  priority_changed: AlertTriangle,
  comment_added: MessageSquare,
};

export default function IssueDetailSpotlight() {
  const [activeTab, setActiveTab] = useState('details');

  return (
    <FeatureSpotlight
      title="Every discussion, decision, and link stays on the ticket."
      description="Comments, priority changes, and linked issues all live in one place — nothing to hunt for in another tool."
    >
      <div className="rounded-2xl border border-white/10 bg-surface p-6 shadow-2xl shadow-black/50 sm:p-8">
        <div className="flex flex-col gap-6 lg:grid lg:grid-cols-[minmax(0,1fr)_320px] lg:items-stretch">
          <div className="flex flex-col gap-6">
            <div className="break-words rounded-xl border border-white/10 bg-white/5 p-6">
              <p className="text-sm text-slate-500">
                <span className="font-medium">#{SPOTLIGHT_TICKET.ticket_number}</span>
                <span aria-hidden> · </span>
                Updated {SPOTLIGHT_TICKET.updated_relative}
              </p>
              <h3 className="mt-2 text-2xl font-bold text-white">{SPOTLIGHT_TICKET.title}</h3>
            </div>

            <div className="flex flex-col gap-4">
              <div role="tablist" className="flex gap-6 overflow-x-auto border-b border-white/10">
                {TABS.map(({ id, label, icon }) => (
                  <TabButton
                    key={id}
                    icon={icon}
                    isActive={activeTab === id}
                    onClick={() => setActiveTab(id)}
                  >
                    {label}
                    {id === 'comments' && (
                      <span className="rounded-full bg-white/10 px-1.5 py-0.5 text-xs text-slate-300">
                        {SPOTLIGHT_COMMENTS.length}
                      </span>
                    )}
                  </TabButton>
                ))}
              </div>

              {activeTab === 'details' && (
                <div className="h-80 overflow-y-auto break-words rounded-xl border border-white/10 bg-white/5 p-6">
                  <DescriptionMarkdown>{SPOTLIGHT_TICKET.description}</DescriptionMarkdown>
                </div>
              )}

              {activeTab === 'comments' && (
                <div className="flex h-80 flex-col gap-4 overflow-y-auto rounded-xl border border-white/10 bg-white/5 p-6">
                  {SPOTLIGHT_COMMENTS.map((comment) => (
                    <div key={comment.id} className="flex flex-col gap-1">
                      <p className="flex items-center gap-2 text-xs text-slate-500">
                        <Avatar name={comment.user_name} seed={comment.user_id} size="sm" />
                        <span>
                          {comment.user_name} · {comment.relative}
                        </span>
                      </p>
                      <p className="text-sm text-slate-200">{comment.content}</p>
                    </div>
                  ))}
                </div>
              )}

              {activeTab === 'links' && (
                <div className="h-80 overflow-y-auto rounded-xl border border-white/10 bg-white/5 p-6">
                  <div className="flex items-center justify-between gap-3 rounded-lg border border-white/10 bg-black/20 px-4 py-3">
                    <div className="flex min-w-0 items-center gap-3">
                      <Badge size="sm" variant="danger">
                        Blocks
                      </Badge>
                      <span className="shrink-0 text-xs font-medium text-slate-500">
                        #{SPOTLIGHT_LINK.other_ticket_number}
                      </span>
                      <span className="truncate text-sm text-white">{SPOTLIGHT_LINK.other_ticket_title}</span>
                      <Badge size="sm" variant={STATUS_VARIANT[SPOTLIGHT_LINK.other_ticket_status]}>
                        {capitalize(SPOTLIGHT_LINK.other_ticket_status)}
                      </Badge>
                      <Badge size="sm" variant={PRIORITY_VARIANT[SPOTLIGHT_LINK.other_ticket_priority]}>
                        {capitalize(SPOTLIGHT_LINK.other_ticket_priority)}
                      </Badge>
                    </div>
                  </div>
                </div>
              )}

              {activeTab === 'activity' && (
                <div className="h-80 overflow-y-auto rounded-xl border border-white/10 bg-white/5 p-6">
                  <ul className="divide-y divide-white/5">
                    {SPOTLIGHT_ACTIVITY.map((entry) => {
                      const Icon = ACTIVITY_ICON[entry.event_type] ?? Sparkles;
                      return (
                        <li key={entry.id} className="flex gap-3 py-3 first:pt-0">
                          <div className="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-white/10 text-slate-400">
                            <Icon className="h-3.5 w-3.5" />
                          </div>
                          <div className="min-w-0 flex-1">
                            <p className="text-sm text-slate-300">
                              <span className="font-medium text-slate-100">{entry.actor_name}</span>{' '}
                              <ActivityText entry={entry} />
                            </p>
                            <p className="mt-0.5 text-xs text-slate-500">{entry.relative}</p>
                          </div>
                        </li>
                      );
                    })}
                  </ul>
                </div>
              )}
            </div>
          </div>

          <aside className="flex min-h-0 flex-col gap-6">
            <div className="rounded-xl border border-white/10 bg-white/5 p-6">
              <dl className="flex flex-col gap-4">
                <div className="flex items-center justify-between gap-4">
                  <dt className="text-sm text-slate-500">Priority</dt>
                  <dd>
                    <Badge size="sm" variant={PRIORITY_VARIANT[SPOTLIGHT_TICKET.priority]}>
                      {capitalize(SPOTLIGHT_TICKET.priority)}
                    </Badge>
                  </dd>
                </div>
                <div className="flex items-center justify-between gap-4">
                  <dt className="text-sm text-slate-500">Status</dt>
                  <dd>
                    <Badge size="sm" variant={STATUS_VARIANT[SPOTLIGHT_TICKET.status]}>
                      {capitalize(SPOTLIGHT_TICKET.status)}
                    </Badge>
                  </dd>
                </div>
                <div className="flex items-center justify-between gap-4">
                  <dt className="shrink-0 text-sm text-slate-500">Reporter</dt>
                  <dd className="flex min-w-0 items-center gap-2">
                    <Avatar name={SPOTLIGHT_TICKET.created_by_name} seed="demo-dana" size="sm" />
                    <span className="truncate text-sm text-slate-200">{SPOTLIGHT_TICKET.created_by_name}</span>
                  </dd>
                </div>
                <div className="flex items-center justify-between gap-4">
                  <dt className="shrink-0 text-sm text-slate-500">Team</dt>
                  <dd className="flex min-w-0 items-center gap-2 text-sm text-slate-200">
                    <Users className="h-4 w-4 shrink-0 text-slate-400" />
                    <span className="truncate">{DEMO_TEAM_NAME}</span>
                  </dd>
                </div>
              </dl>
            </div>
          </aside>
        </div>
      </div>
    </FeatureSpotlight>
  );
}
