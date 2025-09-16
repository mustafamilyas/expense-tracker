import { JSX } from 'solid-js';
import { useLocation, useNavigate } from '@solidjs/router';
import { auth } from '../lib/auth';

export default function Guard(props: { children: JSX.Element }) {
  const loc = useLocation();
  const navigate = useNavigate();
  const token = () => auth.token();

  if (!token()) {
    const redirect = encodeURIComponent(loc.pathname + (loc.search || ''));
    navigate(`/sign-in?redirect=${redirect}`, { replace: true });
    return null;
  }
  return props.children;
}

