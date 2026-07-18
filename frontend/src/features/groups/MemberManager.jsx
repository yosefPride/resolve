import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { MoreVertical } from 'lucide-react';
import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { useNavigate } from 'react-router-dom';
import { addMember, lookupUserByEmail, removeMember, updateMemberRole } from '../../services/groups.service';
import { GROUP_ROLES, isGroupAdmin } from '../../utils/roles';
import { errorMessage } from '../../utils/errors';
import Button from '../../components/ui/Button';
import Input from '../../components/ui/Input';
import Badge from '../../components/ui/Badge';

function AddMemberForm({ groupId }) {
  const queryClient = useQueryClient();
  const [email, setEmail] = useState('');
  const [found, setFound] = useState(null);
  const [error, setError] = useState('');
  const [isLookingUp, setIsLookingUp] = useState(false);

  const addMutation = useMutation({
    mutationFn: ({ userId, role }) => addMember(groupId, userId, role),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['group', groupId, 'members'] });
      setFound(null);
      setEmail('');
    },
    onError: (err) => setError(errorMessage(err, 'Failed to add member.')),
  });

  const isBusy = isLookingUp || addMutation.isPending;

  async function handleLookup(event) {
    event.preventDefault();
    setError('');
    setFound(null);
    setIsLookingUp(true);
    try {
      setFound(await lookupUserByEmail(groupId, email));
    } catch (err) {
      setError(errorMessage(err, 'No user found with that email.'));
    } finally {
      setIsLookingUp(false);
    }
  }

  function handleConfirm(role) {
    setError('');
    addMutation.mutate({ userId: found.id, role });
  }

  return (
    <div className="flex flex-col gap-3">
      <form onSubmit={handleLookup} className="flex gap-2">
        <Input
          type="email"
          value={email}
          onChange={(event) => setEmail(event.target.value)}
          placeholder="Exact email address"
          required
          className="flex-1"
        />
        <Button type="submit" disabled={isBusy}>
          Find
        </Button>
      </form>

      {error && <p className="text-sm text-red-500">{error}</p>}

      {found && (
        <div className="flex items-center justify-between rounded-lg border border-white/10 bg-white/5 px-4 py-3">
          <div>
            <p className="text-sm font-medium text-white">{found.name}</p>
            <p className="text-xs text-slate-400">{found.email}</p>
          </div>
          <div className="flex gap-2">
            <Button
              variant="ghost"
              size="sm"
              disabled={isBusy}
              onClick={() => handleConfirm(GROUP_ROLES.CONTRIBUTOR)}
              className="border border-white/10"
            >
              Add as Contributor
            </Button>
            <Button
              variant="ghost"
              size="sm"
              disabled={isBusy}
              onClick={() => handleConfirm(GROUP_ROLES.GROUP_ADMIN)}
              className="border border-white/10"
            >
              Add as Team Admin
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

function MemberActionsMenu({ member, isSelf, canChangeRole, isBusy, onToggleRole, onRemove }) {
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger
        disabled={isBusy}
        aria-label="Member actions"
        className="flex h-8 w-8 items-center justify-center rounded-full text-slate-400 transition-colors hover:bg-white/10 hover:text-white disabled:opacity-50"
      >
        <MoreVertical className="h-5 w-5" />
      </DropdownMenu.Trigger>

      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={8}
          className="z-50 w-40 rounded-lg border border-white/10 bg-neutral-950 py-1 shadow-2xl shadow-black/50"
        >
          {canChangeRole && (
            <DropdownMenu.Item
              onSelect={onToggleRole}
              className="cursor-pointer px-4 py-2 text-sm text-slate-300 outline-none transition-colors data-highlighted:bg-white/10"
            >
              {isGroupAdmin(member.role) ? 'Demote' : 'Promote'}
            </DropdownMenu.Item>
          )}
          <DropdownMenu.Item
            onSelect={onRemove}
            className="cursor-pointer px-4 py-2 text-sm text-red-400 outline-none transition-colors data-highlighted:bg-white/10"
          >
            {isSelf ? 'Leave' : 'Remove'}
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}

export default function MemberManager({ groupId, members, myUserId, myRole }) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [error, setError] = useState('');
  const [busyId, setBusyId] = useState(null);
  const iAmAdmin = isGroupAdmin(myRole);

  const roleMutation = useMutation({
    mutationFn: ({ userId, role }) => updateMemberRole(groupId, userId, role),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['group', groupId, 'members'] }),
    onError: (err) => setError(errorMessage(err, 'Failed to update role.')),
    onSettled: () => setBusyId(null),
  });

  const removeMutation = useMutation({
    mutationFn: (userId) => removeMember(groupId, userId),
    onSuccess: (_data, userId) => {
      // Removing yourself means you've lost access — leave the page.
      if (userId === myUserId) {
        navigate('/groups');
        return;
      }
      queryClient.invalidateQueries({ queryKey: ['group', groupId, 'members'] });
    },
    onError: (err) => setError(errorMessage(err, 'Failed to remove member.')),
    onSettled: () => setBusyId(null),
  });

  function handleRoleToggle(member) {
    setError('');
    setBusyId(member.user_id);
    const nextRole = isGroupAdmin(member.role) ? GROUP_ROLES.CONTRIBUTOR : GROUP_ROLES.GROUP_ADMIN;
    roleMutation.mutate({ userId: member.user_id, role: nextRole });
  }

  function handleRemove(member) {
    setError('');
    setBusyId(member.user_id);
    removeMutation.mutate(member.user_id);
  }

  return (
    <div className="flex flex-col gap-4">
      {error && <p className="text-sm text-red-500">{error}</p>}

      <ul className="flex flex-col gap-2">
        {members.map((member) => {
          const isSelf = member.user_id === myUserId;
          const isBusy = busyId === member.user_id;
          return (
            <li
              key={member.id}
              className="flex items-center justify-between rounded-lg border border-white/10 bg-white/5 px-4 py-3"
            >
              <div>
                <p className="text-sm font-medium text-white">{member.name}</p>
                <p className="text-xs text-slate-400">{member.email}</p>
              </div>
              <div className="flex items-center gap-2">
                <Badge>{isGroupAdmin(member.role) ? 'Team Admin' : 'Contributor'}</Badge>
                {iAmAdmin && (
                  <MemberActionsMenu
                    member={member}
                    isSelf={isSelf}
                    canChangeRole={iAmAdmin}
                    isBusy={isBusy}
                    onToggleRole={() => handleRoleToggle(member)}
                    onRemove={() => handleRemove(member)}
                  />
                )}
              </div>
            </li>
          );
        })}
      </ul>

      {iAmAdmin && (
        <div className="flex flex-col gap-3 border-t border-white/10 pt-4">
          <h3 className="text-sm font-semibold text-white">Add member</h3>
          <AddMemberForm groupId={groupId} />
        </div>
      )}
    </div>
  );
}
