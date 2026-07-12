import { Link } from 'react-router-dom';

export default function GroupList({ groups }) {
  if (groups.length === 0) {
    return <p className="text-sm text-slate-400">You're not in any groups yet.</p>;
  }

  return (
    <ul className="flex flex-col gap-2">
      {groups.map((group) => (
        <li key={group.id}>
          <Link
            to={`/groups/${group.id}`}
            className="block rounded-lg border border-white/10 bg-white/5 px-4 py-3 text-white transition-colors hover:border-sky-400/50 hover:bg-white/10"
          >
            {group.name}
          </Link>
        </li>
      ))}
    </ul>
  );
}
