import { Route, A, Navigate, useNavigate, useSearchParams } from '@solidjs/router';
import { Show } from 'solid-js';
import { auth } from './lib/auth';
import SignIn from './routes/SignIn';
import Register from './routes/Register';
import Dashboard from './routes/Dashboard';
import ChatBindConfirm from './routes/ChatBindConfirm';
import Guard from './components/Guard';

function Shell(props: { children: any }) {
  const navigate = useNavigate();
  const [params] = useSearchParams();
  return (
    <div class="container py-6">
      <header class="flex items-center justify-between mb-6">
        <A href="/" class="text-xl font-semibold">Expense Tracker</A>
        <div class="flex items-center gap-3">
          <Show when={auth.user()} fallback={<A href={`/sign-in${params.redirect ? `?redirect=${encodeURIComponent(params.redirect)}` : ''}`} class="btn">Sign In</A>}>
            <A href="/" class="text-sm text-gray-600">{auth.user()?.email}</A>
            <button class="btn" onClick={() => { auth.signOut(); navigate('/sign-in'); }}>Sign out</button>
          </Show>
        </div>
      </header>
      {props.children}
    </div>
  );
}

export default function App() {
  return (
    <>
      <Route path="/" component={()=><Navigate href="/dashboard" />} />
      <Route path="/sign-in" component={() => <Shell><SignIn /></Shell>} />
      <Route path="/register" component={() => <Shell><Register /></Shell>} />
      <Route path="/dashboard" component={() => <Shell><Guard><Dashboard /></Guard></Shell>} />
      <Route path="/chat-binding/confirm/:id" component={() => <Shell><Guard><ChatBindConfirm /></Guard></Shell>} />
      <Route path="*" component={()=><Shell><div class="card">Not found</div></Shell>} />
    </>
  );
}
