import { useNavigate, useSearchParams, A } from '@solidjs/router';
import { createSignal, Show } from 'solid-js';
import { api } from '../lib/api';

export default function Register() {
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const [email, setEmail] = createSignal('');
  const [password, setPassword] = createSignal('');
  const [startOverDate, setStartOverDate] = createSignal<number>(1);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [success, setSuccess] = createSignal<string | null>(null);

  async function onSubmit(e: Event) {
    e.preventDefault();
    setLoading(true); setError(null); setSuccess(null);
    try {
      await api.register(email(), password(), startOverDate());
      setSuccess('Registration successful. You can now sign in.');
      const to = `/sign-in${params.redirect ? `?redirect=${encodeURIComponent(params.redirect)}` : ''}`;
      navigate(to, { replace: true });
    } catch (err: any) {
      setError(err.message || 'Registration failed');
    } finally {
      setLoading(false);
    }
  }

  return (
    <div class="card">
      <h1 class="text-2xl font-semibold mb-4">Register</h1>
      <Show when={error()}><div class="mb-3 text-red-600">{error()}</div></Show>
      <Show when={success()}><div class="mb-3 text-green-600">{success()}</div></Show>
      <form onSubmit={onSubmit} class="space-y-4">
        <div>
          <label class="label" for="email">Email</label>
          <input id="email" type="email" class="input" value={email()} onInput={e => setEmail(e.currentTarget.value)} required />
        </div>
        <div>
          <label class="label" for="password">Password</label>
          <input id="password" type="password" class="input" value={password()} onInput={e => setPassword(e.currentTarget.value)} required />
        </div>
        <div>
          <label class="label" for="startOverDate">Start over date (1-28)</label>
          <input id="startOverDate" type="number" min="1" max="28" class="input" value={startOverDate()} onInput={e => setStartOverDate(parseInt(e.currentTarget.value || '1', 10))} required />
        </div>
        <button class="btn" disabled={loading()} type="submit">{loading() ? 'Registering...' : 'Register'}</button>
      </form>
      <p class="mt-4 text-sm text-gray-600">
        Already have an account? <A class="text-blue-600 hover:underline" href={`/sign-in${params.redirect ? `?redirect=${encodeURIComponent(params.redirect)}` : ''}`}>Sign in</A>
      </p>
    </div>
  );
}
