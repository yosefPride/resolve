import * as activity from '../activity/handlers.js';
import * as admin from '../admin/handlers.js';
import * as ai from '../ai/handlers.js';
import * as auth from '../auth/handlers.js';
import * as comment from '../comment/handlers.js';
import * as group from '../group/handlers.js';
import * as link from '../link/handlers.js';
import * as reaction from '../reaction/handlers.js';
import * as reference from '../reference/handlers.js';
import * as ticket from '../ticket/handlers.js';
import { Router } from './router.js';

// Same table, same order as server/routes.rs (all under /api/v1).
export function buildRouter() {
  const router = new Router();
  const scope = (prefix, routes) => {
    for (const [method, path, handler] of routes) router.add(method, `/api/v1${prefix}${path}`, handler);
  };

  scope('/auth', [
    ['POST', '/register', auth.register],
    ['POST', '/login', auth.login],
    ['GET', '/me', auth.me],
    ['PATCH', '/me', auth.updateMe],
    ['POST', '/me/password', auth.changePassword],
    ['POST', '/refresh', auth.refresh],
    ['POST', '/logout', auth.logout],
  ]);

  scope('/groups', [
    ['POST', '', group.createGroup],
    ['GET', '', group.listMyGroups],
    ['GET', '/{id}', group.getGroup],
    ['PATCH', '/{id}', group.renameGroup],
    ['DELETE', '/{id}', group.deleteGroup],
    ['GET', '/{id}/users', group.listMembers],
    ['POST', '/{id}/users', group.addMember],
    ['GET', '/{id}/users/lookup', group.lookupUser],
    ['PATCH', '/{id}/users/{user_id}', group.updateMemberRole],
    ['DELETE', '/{id}/users/{user_id}', group.removeMember],
    ['POST', '/{id}/tickets', ticket.createTicket],
    ['GET', '/{id}/tickets', ticket.listTickets],
    ['GET', '/{id}/tickets/{ticket_id}', ticket.getTicket],
    ['PATCH', '/{id}/tickets/{ticket_id}', ticket.updateTicket],
    ['DELETE', '/{id}/tickets/{ticket_id}', ticket.deleteTicket],
    ['POST', '/{id}/tickets/{ticket_id}/comments', comment.createComment],
    ['GET', '/{id}/tickets/{ticket_id}/comments', comment.listComments],
    ['DELETE', '/{id}/tickets/{ticket_id}/comments/{comment_id}', comment.deleteComment],
    ['PUT', '/{id}/tickets/{ticket_id}/comments/{comment_id}/reactions', reaction.setReaction],
    ['DELETE', '/{id}/tickets/{ticket_id}/comments/{comment_id}/reactions', reaction.removeReaction],
    ['GET', '/{id}/tickets/{ticket_id}/activity', activity.listActivity],
    ['POST', '/{id}/tickets/{ticket_id}/links', link.createLink],
    ['GET', '/{id}/tickets/{ticket_id}/links', link.listLinks],
    ['DELETE', '/{id}/tickets/{ticket_id}/links/{link_id}', link.deleteLink],
    ['POST', '/{id}/tickets/{ticket_id}/references', reference.createReference],
    ['GET', '/{id}/tickets/{ticket_id}/references', reference.listReferences],
    ['DELETE', '/{id}/tickets/{ticket_id}/references/{reference_id}', reference.deleteReference],
  ]);

  scope('/ai/groups/{id}/tickets/{ticket_id}', [
    ['POST', '/summarize', ai.summarizeTicket],
    ['POST', '/analyze', ai.analyzeTicket],
    ['POST', '/conversations', ai.createConversation],
    ['GET', '/conversations', ai.listConversations],
    ['POST', '/conversations/{conversation_id}/messages', ai.sendConversationMessage],
    ['GET', '/conversations/{conversation_id}/messages', ai.listConversationMessages],
    ['DELETE', '/conversations/{conversation_id}', ai.deleteConversation],
  ]);

  scope('/admin', [
    ['GET', '/users', admin.listUsers],
    ['GET', '/groups', admin.listGroups],
    ['DELETE', '/groups/{id}', admin.deleteGroup],
    ['GET', '/audit-log', admin.listAuditLog],
    ['GET', '/users/{id}/deletion-check', admin.deletionCheck],
    ['POST', '/users/{id}/delete', admin.deleteUser],
    ['POST', '/users/{id}/promote', admin.promoteUser],
    ['POST', '/users/{id}/demote', admin.demoteUser],
  ]);

  return router;
}
