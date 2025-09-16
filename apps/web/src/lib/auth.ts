import { createSignal } from 'solid-js';
import type { UserRead } from './api';

type Session = { token: string; user: UserRead } | null;

const STORAGE_KEY = 'et:web:session';

function load(): Session {
  try { return JSON.parse(localStorage.getItem(STORAGE_KEY) || 'null'); } catch { return null; }
}

const [session, setSession] = createSignal<Session>(load());

export const auth = {
  session,
  token: () => session()?.token || '',
  user: () => session()?.user || null,
  signIn: (s: Session) => { setSession(s); localStorage.setItem(STORAGE_KEY, JSON.stringify(s)); },
  signOut: () => { setSession(null); localStorage.removeItem(STORAGE_KEY); },
};

