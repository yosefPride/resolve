import { useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useGroup } from '../hooks/useGroup';
import { useAuth } from '../hooks/useAuth';
import { isGroupAdmin } from '../utils/roles';
import { deleteGroup, removeMember } from '../services/groups.service';
import { errorMessage } from '../utils/errors';
import MemberManager from '../features/groups/MemberManager';
import RenameGroupForm from '../features/groups/RenameGroupForm';
import Modal from '../components/ui/Modal';

export default function GroupManagementPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { group, members, status, refresh } = useGroup(id);
  const [isConfirmingDelete, setIsConfirmingDelete] = useState(false);
  const [deleteError, setDeleteError] = useState('');
  const [isDeleting, setIsDeleting] = useState(false);
  const [isRenaming, setIsRenaming] = useState(false);
  const [isConfirmingLeave, setIsConfirmingLeave] = useState(false);
  const [leaveError, setLeaveError] = useState('');
  const [isLeaving, setIsLeaving] = useState(false);

  if (status === 'loading') {
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
      navigate('/groups');
    } catch (err) {
      setDeleteError(errorMessage(err, 'Failed to delete team.'));
      setIsDeleting(false);
    }
  }

  function handleRenamed() {
    setIsRenaming(false);
    refresh(); // re-fetch so the heading (and members) reflect the new name
  }

  async function handleLeave() {
    setLeaveError('');
    setIsLeaving(true);
    try {
      await removeMember(id, user.id);
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
              <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.75">
                <path strokeLinecap="round" strokeLinejoin="round" d="M16.5 3.5a2.121 2.121 0 013 3L7 19l-4 1 1-4 12.5-12.5z" />
              </svg>
            </button>
            <button
              type="button"
              onClick={() => setIsConfirmingDelete(true)}
              className="rounded-full border border-red-500/50 px-4 py-2 text-sm font-semibold text-red-400 transition-colors hover:bg-red-500/10"
            >
              Delete team
            </button>
          </div>
        ) : (
          <button
            type="button"
            onClick={() => setIsConfirmingLeave(true)}
            className="rounded-full border border-red-500/50 px-4 py-2 text-sm font-semibold text-red-400 transition-colors hover:bg-red-500/10"
          >
            Leave team
          </button>
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
          lose access to its tickets, and a Team Admin would need to add you back to rejoin.
        </p>

        {leaveError && <p className="mt-3 text-sm text-red-500">{leaveError}</p>}

        <div className="mt-6 flex justify-end gap-3">
          <button
            type="button"
            onClick={() => {
              setIsConfirmingLeave(false);
              setLeaveError('');
            }}
            className="rounded-full px-4 py-2 text-sm font-medium text-slate-300 transition-colors hover:bg-white/10 hover:text-white"
          >
            Cancel
          </button>
          <button
            type="button"
            disabled={isLeaving}
            onClick={handleLeave}
            className="rounded-full bg-red-500 px-4 py-2 text-sm font-semibold text-white transition-colors hover:bg-red-600 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {isLeaving ? 'Leaving…' : 'Leave team'}
          </button>
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
          <button
            type="button"
            onClick={() => {
              setIsConfirmingDelete(false);
              setDeleteError('');
            }}
            className="rounded-full px-4 py-2 text-sm font-medium text-slate-300 transition-colors hover:bg-white/10 hover:text-white"
          >
            Cancel
          </button>
          <button
            type="button"
            disabled={isDeleting}
            onClick={handleDelete}
            className="rounded-full bg-red-500 px-4 py-2 text-sm font-semibold text-white transition-colors hover:bg-red-600 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {isDeleting ? 'Deleting…' : 'Delete team'}
          </button>
        </div>
      </Modal>

      <div className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold text-white">Members</h2>
        <MemberManager
          groupId={id}
          members={members}
          myUserId={user.id}
          myRole={myRole}
          onChanged={refresh}
        />
      </div>
    </section>
  );
}
