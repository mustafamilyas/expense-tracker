import { A, useNavigate, useSearchParams } from "@solidjs/router";
import { createSignal, Show } from "solid-js";
import { api } from "../lib/api";
import { auth } from "../lib/auth";

export default function Register() {
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const [email, setEmail] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [success, setSuccess] = createSignal<string | null>(null);

  async function onSubmit(e: Event) {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setSuccess(null);
    try {
      const res = await api.register(email(), password());
      auth.signIn(res);
      setSuccess("Registration successful. You can now sign in.");
      const to = params.redirect
        ? decodeURIComponent(params.redirect)
        : "/dashboard";
      navigate(to, { replace: true });
    } catch (err: any) {
      setError(err.message || "Registration failed");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div class="card">
      <h1 class="text-2xl font-semibold mb-4">Register</h1>
      <Show when={error()}>
        <div class="mb-3 text-red-600">{error()}</div>
      </Show>
      <Show when={success()}>
        <div class="mb-3 text-green-600">{success()}</div>
      </Show>
      <form onSubmit={onSubmit} class="space-y-4">
        <div>
          <label class="label" for="email">
            Email
          </label>
          <input
            id="email"
            type="email"
            class="input"
            value={email()}
            onInput={(e) => setEmail(e.currentTarget.value)}
            required
          />
        </div>
        <div>
          <label class="label" for="password">
            Password
          </label>
          <input
            id="password"
            type="password"
            class="input"
            value={password()}
            onInput={(e) => setPassword(e.currentTarget.value)}
            required
          />
        </div>
        <button class="btn" disabled={loading()} type="submit">
          {loading() ? "Registering..." : "Register"}
        </button>
      </form>
      <p class="mt-4 text-sm text-gray-600">
        Already have an account?{" "}
        <A
          class="text-blue-600 hover:underline"
          href={`/sign-in${
            params.redirect
              ? `?redirect=${encodeURIComponent(params.redirect)}`
              : ""
          }`}
        >
          Sign in
        </A>
      </p>
    </div>
  );
}
