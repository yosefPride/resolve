import { useCallback, useEffect, useState } from 'react';
import { getGroup, listMembers } from '../services/groups.service';

export function useGroup(groupId) {
  const [group, setGroup] = useState(null);
  const [members, setMembers] = useState([]);
  const [status, setStatus] = useState('loading');

  const refresh = useCallback(() => {
    return Promise.all([getGroup(groupId), listMembers(groupId)])
      .then(([groupData, memberData]) => {
        setGroup(groupData);
        setMembers(memberData);
        setStatus('ready');
      })
      .catch(() => {
        setStatus('error');
      });
  }, [groupId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { group, members, status, refresh };
}
