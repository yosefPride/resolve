import { useQuery } from '@tanstack/react-query';
import { listActivity } from '../services/activity.service';

export function useActivity(groupId, ticketId) {
  return useQuery({
    queryKey: ['activity', groupId, ticketId],
    queryFn: () => listActivity(groupId, ticketId),
    enabled: Boolean(groupId) && Boolean(ticketId),
  });
}
