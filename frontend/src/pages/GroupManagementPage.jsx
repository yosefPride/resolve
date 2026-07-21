import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Pencil } from 'lucide-react';
import { useParams, useNavigate } from 'react-router-dom';
import { useGroup } from '../hooks/useGroup';
import { useAuth } from '../hooks/useAuth';
import { isGroupAdmin } from '../utils/roles';
import { deleteGroup, removeMember } from '../services/groups.service';
import { errorMessage } from '../utils/errors';
import MemberManager from '../features/groups/MemberManager';
import RenameGroupForm from '../features/groups/RenameGroupForm';
import Modal from '../components/ui/Modal';
import Button from '../components/ui/Button';

export default function GroupManagementPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const { user } = useAuth();
  const queryClient = useQueryClient();
  const { group, members, status } = useGroup(id);
  const [isConfirmingDelete, setIsConfirmingDelete] = useState(false);
  const [deleteError, setDeleteError] = useState('');
  const [isDeleting, setIsDeleting] = useState(false);
  const [isRenaming, setIsRenaming] = useState(false);
  const [isConfirmingLeave, setIsConfirmingLeave] = useState(false);
  const [leaveError, setLeaveError] = useState('');
  const [isLeaving, setIsLeaving] = useState(false);

  if (status === 'pending') {
    return (
      <section className="mx-auto max-w-2xl px-4 py-20 sm:px-6 lg:px-8">
        <p className="text-sm text-slate-400">Loading…</p>
      </section>
    );
  }

  if (status === 'error') {
    return (
      <section className="mx-auto max-w-2xl px-4 py-20 sm:px-6 lg:px-8">
        <p className="text-sm text-red-500">
          Couldn't load this team. You may not be a member, or it may not exist.
        </p>
      </section>
    );
  }

  const myRole = members.find((member) => member.user_id === user.id)?.role;
  const iAmAdmin = isGroupAdmin(myRole);

  async function handleDelete() {
    setDeleteError('');
    setIsDeleting(true);
    try {
      await deleteGroup(id);
      queryClient.invalidateQueries({ queryKey: ['groups'] });
      navigate('/groups');
    } catch (err) {
      setDeleteError(errorMessage(err, 'Failed to delete team.'));
      setIsDeleting(false);
    }
  }

  function handleRenamed() {
    setIsRenaming(false);
    // Refresh the heading here and the teams list (name shown there too).
    queryClient.invalidateQueries({ queryKey: ['group', id] });
    queryClient.invalidateQueries({ queryKey: ['groups'] });
  }

  async function handleLeave() {
    setLeaveError('');
    setIsLeaving(true);
    try {
      await removeMember(id, user.id);
      queryClient.invalidateQueries({ queryKey: ['groups'] });
      navigate('/groups');
    } catch (err) {
      setLeaveError(errorMessage(err, 'Failed to leave team.'));
      setIsLeaving(false);
    }
  }

  return (
    <section className="mx-auto flex max-w-2xl flex-col gap-8 px-4 py-20 sm:px-6 lg:px-8">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">{group.name}</h1>
          {myRole && (
            <p className="text-sm text-slate-400">
              Your role: {isGroupAdmin(myRole) ? 'Team Admin' : 'Contributor'}
            </p>
          )}
        </div>
        {iAmAdmin ? (
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => setIsRenaming(true)}
              aria-label="Rename team"
              title="Rename team"
              className="flex h-9 w-9 items-center justify-center rounded-full bg-white/10 text-slate-300 transition-colors hover:bg-white/20 hover:text-white"
            >
              <Pencil className="h-4 w-4" />
            </button>
            <Button variant="dangerOutline" onClick={() => setIsConfirmingDelete(true)}>
              Delete team
            </Button>
          </div>
        ) : (
          <Button variant="dangerOutline" onClick={() => setIsConfirmingLeave(true)}>
            Leave team
          </Button>
        )}
      </div>

      <Modal
        isOpen={isRenaming}
        onClose={() => setIsRenaming(false)}
        title="Rename team"
      >
        <RenameGroupForm groupId={id} currentName={group.name} onRenamed={handleRenamed} />
      </Modal>

      <Modal
        isOpen={isConfirmingLeave}
        onClose={() => {
          setIsConfirmingLeave(false);
          setLeaveError('');
        }}
        title="Leave team"
      >
        <p className="text-sm text-slate-300">
          Are you sure you want to leave <span className="font-semibold text-white">{group.name}</span>? You'll
          lose access to its issues, and a Team Admin would need to add you back to rejoin.
        </p>

        {leaveError && <p className="mt-3 text-sm text-red-500">{leaveError}</p>}

        <div className="mt-6 flex justify-end gap-3">
          <Button
            variant="ghost"
            onClick={() => {
              setIsConfirmingLeave(false);
              setLeaveError('');
            }}
          >
            Cancel
          </Button>
          <Button variant="danger" disabled={isLeaving} onClick={handleLeave}>
            {isLeaving ? 'Leaving…' : 'Leave team'}
          </Button>
        </div>
      </Modal>

      <Modal
        isOpen={isConfirmingDelete}
        onClose={() => {
          setIsConfirmingDelete(false);
          setDeleteError('');
        }}
        title="Delete team"
      >
        <p className="text-sm text-slate-300">
          Are you sure you want to delete <span className="font-semibold text-white">{group.name}</span>? This
          cannot be undone.
        </p>

        {deleteError && <p className="mt-3 text-sm text-red-500">{deleteError}</p>}

        <div className="mt-6 flex justify-end gap-3">
          <Button
            variant="ghost"
            onClick={() => {
              setIsConfirmingDelete(false);
              setDeleteError('');
            }}
          >
            Cancel
          </Button>
          <Button variant="danger" disabled={isDeleting} onClick={handleDelete}>
            {isDeleting ? 'Deleting…' : 'Delete team'}
          </Button>
        </div>
      </Modal>

      <div className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold text-white">Members</h2>
        <MemberManager
          groupId={id}
          members={members}
          myUserId={user.id}
          myRole={myRole}
        />
      </div>
    </section>
  );
}
