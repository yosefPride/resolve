import { useQuery } from '@tanstack/react-query';
import { getGroup, listMembers } from '../services/groups.service';

// Two parallel queries under a shared ['group', groupId] prefix. Member
// mutations (add / remove / role change) invalidate ['group', groupId,
// 'members'] to refresh the list — no manual refresh plumbing needed.
export function useGroup(groupId) {
  const groupQuery = useQuery({
    queryKey: ['group', groupId],
    queryFn: () => getGroup(groupId),
  });
  const membersQuery = useQuery({
    queryKey: ['group', groupId, 'members'],
    queryFn: () => listMembers(groupId),
  });

  const status =
    groupQuery.isPending || membersQuery.isPending
      ? 'pending'
      : groupQuery.isError || membersQuery.isError
        ? 'error'
        : 'success';

  return {
    group: groupQuery.data ?? null,
    members: membersQuery.data ?? [],
    status,
  };
}
