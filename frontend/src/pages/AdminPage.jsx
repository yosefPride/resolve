import { useState } from 'react';
import UsersPanel from '../features/admin/UsersPanel';
import GroupsPanel from '../features/admin/GroupsPanel';
import AuditLogPanel from '../features/admin/AuditLogPanel';

const TABS = [
  { id: 'users', label: 'Users' },
  { id: 'groups', label: 'Groups' },
  { id: 'audit', label: 'Audit Log' },
];

export default function AdminPage() {
  const [activeTab, setActiveTab] = useState('users');

  return (
    <section className="mx-auto flex max-w-7xl flex-col gap-6 px-4 py-20 sm:px-6 lg:px-8">
      <h1 className="text-2xl font-bold text-white">Admin</h1>

      <div role="tablist" aria-label="Admin sections" className="flex gap-2 border-b border-white/10">
        {TABS.map((tab) => {
          const isActive = activeTab === tab.id;
          return (
            <button
              key={tab.id}
              type="button"
              role="tab"
              aria-selected={isActive}
              onClick={() => setActiveTab(tab.id)}
              className={`-mb-px rounded-t-lg px-4 py-2 text-sm font-medium transition-colors ${
                isActive
                  ? 'border-b-2 border-white text-white'
                  : 'text-gray-400 hover:text-white'
              }`}
            >
              {tab.label}
            </button>
          );
        })}
      </div>

      <div role="tabpanel">
        {activeTab === 'users' && <UsersPanel />}
        {activeTab === 'groups' && <GroupsPanel />}
        {activeTab === 'audit' && <AuditLogPanel />}
      </div>
    </section>
  );
}
