import { Link } from 'react-router-dom';
import { User, LogOut } from 'lucide-react';
import * as DropdownMenu from '@radix-ui/react-dropdown-menu';

export default function UserMenu({ user, onLogout }) {
  const isSystemAdmin = user?.global_role === 'SystemAdmin';

  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger
        title="Open user navigation menu"
        aria-label="Open user navigation menu"
        className="flex h-10 w-10 items-center justify-center rounded-full border border-white/10 bg-white/5 text-slate-300 transition-all duration-200 hover:bg-white/10 hover:ring-1 hover:ring-white focus:ring-1 focus:ring-white focus:scale-95"
      >
        <User className="h-7 w-7" strokeWidth={1.5} />
      </DropdownMenu.Trigger>

      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={8}
          className="z-50 w-56 rounded-lg border border-white/10 bg-neutral-950 py-1 shadow-2xl shadow-black/50"
        >
          <DropdownMenu.Label className="flex items-center gap-2 px-4 py-3">
            <span className="relative flex h-2 w-2 shrink-0">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-300 opacity-75" />
              <span className="relative inline-flex h-2 w-2 rounded-full bg-green-300" />
            </span>
            <span className="truncate text-sm font-medium text-green-400">{user?.name}</span>
          </DropdownMenu.Label>

          <DropdownMenu.Separator className="h-px bg-white/10" />

          <DropdownMenu.Item asChild>
            <Link
              to="/account"
              className="block cursor-pointer px-4 py-2 text-sm text-slate-300 outline-none transition-colors data-highlighted:bg-white/10"
            >
              Account
            </Link>
          </DropdownMenu.Item>

          {isSystemAdmin && (
            <DropdownMenu.Item asChild>
              <Link
                to="/admin"
                className="block cursor-pointer px-4 py-2 text-sm text-slate-300 outline-none transition-colors data-highlighted:bg-white/10"
              >
                Admin
              </Link>
            </DropdownMenu.Item>
          )}

          <DropdownMenu.Separator className="h-px bg-white/10" />

          <DropdownMenu.Item
            onSelect={onLogout}
            className="flex w-full cursor-pointer items-center gap-2 px-4 py-2 text-left text-sm text-slate-300 outline-none transition-colors data-highlighted:bg-white/10"
          >
            <LogOut className="h-4 w-4" />
            Log out
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
