import { writable } from 'svelte/store';

export interface Route {
  page: 'home' | 'tool' | 'notfound';
  params: Record<string, string>;
}

function parseHash(): Route {
  const hash = window.location.hash.slice(1) || '/';

  if (hash === '/') {
    return { page: 'home', params: {} };
  }

  const match = hash.match(/^\/tool\/(.+)$/);
  if (match) {
    return { page: 'tool', params: { id: decodeURIComponent(match[1]) } };
  }

  return { page: 'notfound', params: {} };
}

export const currentRoute = writable<Route>(parseHash());

export function navigateTo(path: string) {
  window.location.hash = path;
}

if (typeof window !== 'undefined') {
  window.addEventListener('hashchange', () => {
    currentRoute.set(parseHash());
  });
}
