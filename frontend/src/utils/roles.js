// Matches the API's actual serialized values (backend/src/group/models.rs's
// Role is #[serde(rename_all = "snake_case")]; GlobalRole has no rename).
export const GROUP_ROLES = {
  CONTRIBUTOR: 'contributor',
  GROUP_ADMIN: 'group_admin',
};

export function isGroupAdmin(role) {
  return role === GROUP_ROLES.GROUP_ADMIN;
}

export function isSystemAdmin(user) {
  return user?.global_role === 'SystemAdmin';
}
