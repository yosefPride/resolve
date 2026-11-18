import { datetime, model, objectId, oneOf, option, string } from '../db/model.js';
import { formatDateTime } from '../rust.js';

export const GlobalRole = { SystemAdmin: 'SystemAdmin' };

export const decodeUser = model('User', {
  _id: option(objectId),
  email: string,
  password_hash: string,
  name: string,
  global_role: option(oneOf(['SystemAdmin'])),
  created_at: datetime,
});

// UserResponse — never includes password_hash.
export function userResponse(user) {
  return {
    id: user._id ? user._id.toHexString() : '',
    email: user.email,
    name: user.name,
    global_role: user.global_role,
    created_at: formatDateTime(user.created_at),
  };
}
