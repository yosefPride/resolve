import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { addMember, lookupUserByEmail, removeMember, updateMemberRole } from '../../services/groups.service';
import { GROUP_ROLES, isGroupAdmin } from '../../utils/roles';

function extractError(err, fallback) {
  return err.response?.data?.error?.message || fallback;
}

function AddMemberForm({ groupId, onAdded }) {
  const [email, setEmail] = useState('');
  const [found, setFound] = useState(null);
  const [error, setError] = useState('');
  const [isBusy, setIsBusy] = useState(false);

  async function handleLookup(event) {
    event.preventDefault();
    setError('');
    setFound(null);
    setIsBusy(true);
    try {
      const user = await lookupUserByEmail(groupId, email);
      setFound(user);
    } catch (err) {
      setError(extractError(err, 'No user found with that email.'));
    } finally {
      setIsBusy(false);
    }
  }

  async function handleConfirm(role) {
    setError('');
    setIsBusy(true);
    try {
      await addMember(groupId, found.id, role);
      setFound(null);
      setEmail('');
      onAdded();
    } catch (err) {
      setError(extractError(err, 'Failed to add member.'));
    } finally {
      setIsBusy(false);
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <form onSubmit={handleLookup} className="flex gap-2">
        <input
          type="email"
          value={email}
          onChange={(event) => setEmail(event.target.value)}
          placeholder="Exact email address"
          required
          className="flex-1 rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50"
        />
        <button
          type="submit"
          disabled={isBusy}
          className="rounded-full bg-white px-4 py-2 text-sm font-semibold text-black transition-all duration-200 hover:bg-black hover:ring-1 hover:ring-white hover:text-white disabled:cursor-not-allowed disabled:bg-white/50 disabled:text-black/50"
        >
          Find
        </button>
      </form>

      {error && <p className="text-sm text-red-500">{error}</p>}

      {found && (
        <div className="flex items-center justify-between rounded-lg border border-white/10 bg-white/5 px-4 py-3">
          <div>
            <p className="text-sm font-medium text-white">{found.name}</p>
            <p className="text-xs text-slate-400">{found.email}</p>
          </div>
          <div className="flex gap-2">
            <button
              type="button"
              disabled={isBusy}
              onClick={() => handleConfirm(GROUP_ROLES.CONTRIBUTOR)}
              className="rounded-full border border-white/10 px-3 py-1 text-xs font-medium text-slate-300 transition-colors hover:bg-white/10 disabled:opacity-50"
            >
              Add as Contributor
            </button>
            <button
              type="button"
              disabled={isBusy}
              onClick={() => handleConfirm(GROUP_ROLES.GROUP_ADMIN)}
              className="rounded-full border border-white/10 px-3 py-1 text-xs font-medium text-slate-300 transition-colors hover:bg-white/10 disabled:opacity-50"
            >
              Add as Group Admin
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// Mirrors UserMenu.jsx's dropdown pattern (own open state + outside-click close).
function MemberActionsMenu({ member, isSelf, canChangeRole, isBusy, onToggleRole, onRemove }) {
  const [isOpen, setIsOpen] = useState(false);
  const menuRef = useRef(null);

  useEffect(() => {
    function handleClickOutside(event) {
      if (menuRef.current && !menuRef.current.contains(event.target)) {
        setIsOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  return (
    <div className="relative" ref={menuRef}>
      <button
        type="button"
        disabled={isBusy}
        onClick={() => setIsOpen((open) => !open)}
        aria-label="Member actions"
        aria-haspopup="true"
        aria-expanded={isOpen}
        className="flex h-8 w-8 items-center justify-center rounded-full text-slate-400 transition-colors hover:bg-white/10 hover:text-white disabled:opacity-50"
      >
        <svg viewBox="0 0 24 24" className="h-5 w-5" fill="currentColor">
          <circle cx="12" cy="5" r="1.5" />
          <circle cx="12" cy="12" r="1.5" />
          <circle cx="12" cy="19" r="1.5" />
        </svg>
      </button>

      {isOpen && (
        <div className="absolute right-0 z-10 mt-2 w-40 rounded-lg border border-white/10 bg-neutral-950 py-1 shadow-2xl shadow-black/50">
          {canChangeRole && (
            <button
              type="button"
              onClick={() => {
                setIsOpen(false);
                onToggleRole();
              }}
              className="block w-full px-4 py-2 text-left text-sm text-slate-300 transition-colors hover:bg-white/10"
            >
              {isGroupAdmin(member.role) ? 'Demote' : 'Promote'}
            </button>
          )}
          <button
            type="button"
            onClick={() => {
              setIsOpen(false);
              onRemove();
            }}
            className="block w-full px-4 py-2 text-left text-sm text-red-400 transition-colors hover:bg-white/10"
          >
            {isSelf ? 'Leave' : 'Remove'}
          </button>
        </div>
      )}
    </div>
  );
}

export default function MemberManager({ groupId, members, myUserId, myRole, onChanged }) {
  const navigate = useNavigate();
  const [error, setError] = useState('');
  const [busyId, setBusyId] = useState(null);
  const iAmAdmin = isGroupAdmin(myRole);

  async function handleRoleToggle(member) {
    setError('');
    setBusyId(member.user_id);
    const nextRole = isGroupAdmin(member.role) ? GROUP_ROLES.CONTRIBUTOR : GROUP_ROLES.GROUP_ADMIN;
    try {
      await updateMemberRole(groupId, member.user_id, nextRole);
      onChanged();
    } catch (err) {
      setError(extractError(err, 'Failed to update role.'));
    } finally {
      setBusyId(null);
    }
  }

  async function handleRemove(member) {
    setError('');
    setBusyId(member.user_id);
    try {
      await removeMember(groupId, member.user_id);
      if (member.user_id === myUserId) {
        navigate('/groups');
        return;
      }
      onChanged();
    } catch (err) {
      setError(extractError(err, 'Failed to remove member.'));
      setBusyId(null);
    }
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
                <span className="rounded-full bg-white/10 px-3 py-1 text-xs font-medium text-slate-300">
                  {isGroupAdmin(member.role) ? 'Group Admin' : 'Contributor'}
                </span>
                {(iAmAdmin || isSelf) && (
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
          <AddMemberForm groupId={groupId} onAdded={onChanged} />
        </div>
      )}
    </div>
  );
}
