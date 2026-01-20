export function generateId(): string {
  return Math.random().toString(36).substring(2, 15) + Math.random().toString(36).substring(2, 15);
}

export function shortId(): string {
  return Date.now().toString(36) + Math.random().toString(36).substring(2, 5);
}
