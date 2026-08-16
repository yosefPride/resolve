import { useQuery } from '@tanstack/react-query';
import { fetchEmojiCategory } from '../services/emoji.service';

// staleTime/gcTime: Infinity — EmojiHub's category lists are fixed for the
// life of the session, so once a category is fetched there's no reason to
// ever hit the network for it again. `enabled` lets the picker defer the
// request until a category tab is actually opened, instead of fetching all
// three up front.
export function useEmojiCategory(category, enabled) {
  return useQuery({
    queryKey: ['emoji-category', category],
    queryFn: () => fetchEmojiCategory(category),
    enabled,
    staleTime: Infinity,
    gcTime: Infinity,
  });
}
