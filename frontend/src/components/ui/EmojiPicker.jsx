import { useState } from 'react';
import { Smile } from 'lucide-react';
import { useEmojiCategory } from '../../hooks/useEmojiCategory';
import DropdownMenu from './DropdownMenu';

const CATEGORIES = [
  { key: 'smileys-and-people', label: 'Smileys & People' },
  { key: 'animals-and-nature', label: 'Animals & Nature' },
  { key: 'food-and-drink', label: 'Food & Drink' },
];

// Shared emoji-picking popover for both call sites: the comment composer
// (inserting emoji into text — picking one should leave the panel open, so
// several can be added in a row) and the reaction bar (picking one *is* the
// whole action, so the panel should close right away). `closeOnSelect` is
// the only thing that differs between them.
export default function EmojiPicker({ onSelect, closeOnSelect = false, triggerClassName = '' }) {
  const [open, setOpen] = useState(false);
  const [category, setCategory] = useState(CATEGORIES[0].key);
  // Deferred until the panel is actually open, so opening it fetches only
  // the active tab's category instead of all three up front.
  const { data: emoji, status } = useEmojiCategory(category, open);

  function handlePick(char) {
    onSelect(char);
    if (closeOnSelect) setOpen(false);
  }

  return (
    <DropdownMenu
      open={open}
      onOpenChange={setOpen}
      width="w-72"
      trigger={
        <button
          type="button"
          aria-label="Add emoji"
          className={`inline-flex items-center justify-center rounded-full p-1.5 text-slate-400 hover:bg-white/10 hover:text-white ${triggerClassName}`}
        >
          <Smile size={16} />
        </button>
      }
    >
      <div className="flex border-b border-white/10">
        {CATEGORIES.map((c) => (
          <button
            key={c.key}
            type="button"
            onClick={() => setCategory(c.key)}
            className={`flex-1 px-2 py-2 text-xs font-medium transition-colors ${
              category === c.key
                ? 'border-b-2 border-sky-400 text-white'
                : 'text-slate-400 hover:text-white'
            }`}
          >
            {c.label}
          </button>
        ))}
      </div>

      <div className="grid max-h-56 grid-cols-8 gap-1 overflow-y-auto p-2">
        {status === 'pending' && (
          <div className="col-span-8 flex justify-center py-6">
            <div className="h-5 w-5 animate-spin rounded-full border-2 border-white/10 border-t-white" />
          </div>
        )}
        {status === 'error' && (
          <p className="col-span-8 py-4 text-center text-xs text-red-400">Couldn't load emoji.</p>
        )}
        {status === 'success' &&
          emoji.map((e) => (
            <button
              key={e.name}
              type="button"
              title={e.name}
              onClick={() => handlePick(e.char)}
              className="rounded-md p-1 text-lg leading-none hover:bg-white/10"
            >
              {e.char}
            </button>
          ))}
      </div>
    </DropdownMenu>
  );
}
