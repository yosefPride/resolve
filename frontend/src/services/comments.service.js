import api from '../lib/axios';

// Returns the whole thread as a flat, oldest-first array — comments are not
// paginated the way tickets are. Replies carry a parent_comment_id; the tree
// itself is assembled client-side (see CommentList).
export function listComments(groupId, ticketId) {
  return api.get(`/groups/${groupId}/tickets/${ticketId}/comments`).then((res) => res.data);
}

// parentCommentId omitted/null = a top-level comment on the ticket; set = a
// reply to that comment. The backend rejects a parent that isn't a comment on
// this same ticket.
export function createComment(groupId, ticketId, { content, parentCommentId }) {
  return api
    .post(`/groups/${groupId}/tickets/${ticketId}/comments`, {
      content,
      parent_comment_id: parentCommentId ?? null,
    })
    .then((res) => res.data);
}

export function deleteComment(groupId, ticketId, commentId) {
  return api
    .delete(`/groups/${groupId}/tickets/${ticketId}/comments/${commentId}`)
    .then((res) => res.data);
}
