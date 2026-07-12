import { useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useGroup } from '../hooks/useGroup';
import { useAuth } from '../hooks/useAuth';
import { isGroupAdmin } from '../utils/roles';
import { deleteGroup } from '../services/groups.service';
import MemberManager from '../features/groups/MemberManager';
import Modal from '../components/ui/Modal';

export default function GroupManagementPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { group, members, status, refresh } = useGroup(id);
  const [isConfirmingDelete, setIsConfirmingDelete] = useState(false);
  const [deleteError, setDeleteError] = useState('');
  const [isDeleting, setIsDeleting] = useState(false);

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
          Couldn't load this group. You may not be a member, or it may not exist.
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
      setDeleteError(err.response?.data?.error?.message || 'Failed to delete group.');
      setIsDeleting(false);
    }
  }

  return (
    <section className="mx-auto flex max-w-2xl flex-col gap-8 px-4 py-20 sm:px-6 lg:px-8">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">{group.name}</h1>
          {myRole && (
            <p className="text-sm text-slate-400">
              Your role: {isGroupAdmin(myRole) ? 'Group Admin' : 'Contributor'}
            </p>
          )}
        </div>
        {iAmAdmin && (
          <button
            type="button"
            onClick={() => setIsConfirmingDelete(true)}
            className="rounded-full border border-red-500/50 px-4 py-2 text-sm font-semibold text-red-400 transition-colors hover:bg-red-500/10"
          >
            Delete group
          </button>
        )}
      </div>

      <Modal
        isOpen={isConfirmingDelete}
        onClose={() => {
          setIsConfirmingDelete(false);
          setDeleteError('');
        }}
        title="Delete group"
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
            {isDeleting ? 'Deleting…' : 'Delete group'}
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
