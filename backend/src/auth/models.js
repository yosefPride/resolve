import { datetime, model, objectId, option, string } from '../db/model.js';
import * as t from '../serde/types.js';

export const RegisterRequest = t.struct('RegisterRequest', {
  email: t.string,
  password: t.string,
  name: t.string,
});

export const LoginRequest = t.struct('LoginRequest', { email: t.string, password: t.string });

// Body for PATCH /auth/me. Both fields optional so a client can update either
// alone; current_password is only demanded when the email actually changes,
// since email is the login identity and the key Group Admins add members by.
export const UpdateMeRequest = t.struct('UpdateMeRequest', {
  name: t.option(t.string),
  email: t.option(t.string),
  current_password: t.option(t.string),
});

// Body for POST /auth/me/password.
export const ChangePasswordRequest = t.struct('ChangePasswordRequest', {
  current_password: t.string,
  new_password: t.string,
});

// A single refresh-token session. One document per outstanding refresh token;
// revoked_at is set on rotation (single-use) or logout, and expires_at is
// backed by a TTL index so spent/expired rows are cleaned up automatically.
export const decodeRefreshToken = model('RefreshTokenDoc', {
  _id: option(objectId),
  user_id: objectId,
  token_hash: string,
  created_at: datetime,
  expires_at: datetime,
  revoked_at: option(datetime),
});
