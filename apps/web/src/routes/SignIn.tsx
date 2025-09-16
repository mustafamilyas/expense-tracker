import { useNavigate, useSearchParams, A } from '@solidjs/router';
import { createSignal, Show } from 'solid-js';
import { api } from '../lib/api';
import { auth } from '../lib/auth';

export default function SignIn() {
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const [email, setEmail] = createSignal('');
  const [password, setPassword] = createSignal('');
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  async function onSubmit(e: Event) {
    e.preventDefault();
    setLoading(true); setError(null);
    try {
      const res = await api.login(email(), password());
      auth.signIn(res);
      const to = params.redirect ? decodeURIComponent(params.redirect) : '/dashboard';
      navigate(to, { replace: true });
    } catch (err: any) {
      setError(err.message || 'Login failed');
    } finally {
      setLoading(false);
    }
  }

  return (
    <div class="card">
      <h1 class="text-2xl font-semibold mb-4">Sign in</h1>
      <Show when={error()}><div class="mb-3 text-red-600">{error()}</div></Show>
      <form onSubmit={onSubmit} class="space-y-4">
        <div>
          <label class="label" for="email">Email</label>
          <input id="email" type="email" class="input" value={email()} onInput={e => setEmail(e.currentTarget.value)} required />
        </div>
        <div>
          <label class="label" for="password">Password</label>
          <input id="password" type="password" class="input" value={password()} onInput={e => setPassword(e.currentTarget.value)} required />
        </div>
        <button class="btn" disabled={loading()} type="submit">{loading() ? 'Signing in...' : 'Sign in'}</button>
      </form>
      <p class="mt-4 text-sm text-gray-600">
        No account? <A class="text-blue-600 hover:underline" href={`/register${params.redirect ? `?redirect=${encodeURIComponent(params.redirect)}` : ''}`}>Register</A>
      </p>
    </div>
  );
}
