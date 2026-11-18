// Declares a route handler: its extractors (in the Rust handler's argument
// order) and the function that receives their values.
export function handler(extractors, fn) {
  return { extractors, fn };
}
