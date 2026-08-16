import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Plus, UserPlus } from 'lucide-react';
import { listGroups } from '../../services/groups.service';
import { useAuth } from '../../hooks/useAuth';
import Button from '../../components/ui/Button';
import Modal from '../../components/ui/Modal';
import CreateGroupForm from '../groups/CreateGroupForm';
import NewTicketForm from './NewTicketForm';

// Shares the ['groups'] query key with DashboardStats/Sidebar (see
// DashboardStats), so this is normally a cache hit, not a fresh request.
export default function DashboardHeader() {
  const { user } = useAuth();
  const [isCreatingTeam, setIsCreatingTeam] = useState(false);
  const [isCreatingTicket, setIsCreatingTicket] = useState(false);
  const queryClient = useQueryClient();
  const { data: groups = [] } = useQuery({ queryKey: ['groups'], queryFn: listGroups });

  function handleTeamCreated() {
    queryClient.invalidateQueries({ queryKey: ['groups'] });
    setIsCreatingTeam(false);
  }

  return (
    <div className="flex flex-wrap items-center justify-between gap-4">
      <h1 className="text-2xl font-bold text-white">Welcome, {user?.name}</h1>

      <div className="flex shrink-0 items-center gap-2">
        <Button variant="ghost" onClick={() => setIsCreatingTeam(true)}>
          <UserPlus className="mr-1 h-4 w-4" />
          New team
        </Button>
        {groups.length > 0 && (
          <Button onClick={() => setIsCreatingTicket(true)}>
            <Plus className="mr-1 h-4 w-4" />
            New issue
          </Button>
        )}
      </div>

      <Modal isOpen={isCreatingTeam} onClose={() => setIsCreatingTeam(false)} title="Create a team">
        <CreateGroupForm onCreated={handleTeamCreated} />
      </Modal>

      <Modal isOpen={isCreatingTicket} onClose={() => setIsCreatingTicket(false)} title="New issue">
        <NewTicketForm groups={groups} onCreated={() => setIsCreatingTicket(false)} />
      </Modal>
    </div>
  );
}
