// Intentionally empty: auth currently persists nothing of its own — register/login/me
// only read and write `User` documents via `UserRepository`. This file becomes real
// once auth owns data that isn't a `User`, e.g. a revoked-token collection for logout.
