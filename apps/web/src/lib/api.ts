import { auth } from "./auth";

const base = ""; // same-origin; vite dev proxies /api routes to Axum

type HttpMethod = "GET" | "POST" | "PUT" | "DELETE";

async function request<T>(
  path: string,
  options: { method?: HttpMethod; body?: any } = {}
): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };
  const token = auth.token();
  if (token) headers["Authorization"] = `Bearer ${token}`;

  const res = await fetch(base + path, {
    method: options.method || "GET",
    headers,
    body: options.body ? JSON.stringify(options.body) : undefined,
  });

  if (!res.ok) {
    let msg = `${res.status} ${res.statusText}`;
    try {
      const data = await res.json();
      msg = data.message || msg;
    } catch {}
    throw new Error(msg);
  }
  return (await res.json()) as T;
}

interface LoginResponse {
  token: string;
  user: UserRead;
}

export const api = {
  // Auth
  login: (email: string, password: string) =>
    request<LoginResponse>(`/auth/login`, {
      method: "POST",
      body: { email, password },
    }),
  register: (email: string, password: string, start_over_date: number) =>
    request<LoginResponse>(`/auth/register`, {
      method: "POST",
      body: { email, password, start_over_date },
    }),

  // Users
  getUser: (uid: string) => request<UserRead>(`/users/${uid}`),

  // Expense groups
  listGroups: () => request<ExpenseGroup[]>(`/expense-groups`),

  // Chat bind requests
  getChatBindRequest: (id: string) =>
    request<ChatBindRequest>(`/chat-bind-requests/${id}`),

  // Chat bindings
  createChatBinding: (body: CreateChatBindingPayload) =>
    request<ChatBinding>(`/chat-bindings/accept`, { method: "POST", body }),
};

// Types from backend
export type UserRead = { uid: string; email: string; start_over_date: number };
export type ExpenseGroup = {
  uid: string;
  name: string;
  owner: string;
  created_at: string;
};
export type ChatBindRequest = {
  id: string;
  platform: string;
  p_uid: string;
  nonce: string;
  user_uid: string | null;
  expires_at: string;
  created_at: string;
};
export type ChatBinding = {
  id: string;
  group_uid: string;
  platform: string;
  p_uid: string;
  status: string;
  bound_by: string;
  bound_at: string;
  revoked_at: string | null;
};
export type CreateChatBindingPayload = {
  request_id: string;
  nonce: string;
  group_uid: string;
};
