import { useParams } from "@solidjs/router";
import { createResource, createSignal, For, Show } from "solid-js";
import { api, type ChatBindRequest, type ExpenseGroup } from "../lib/api";
import { auth } from "../lib/auth";

export default function ChatBindConfirm() {
  const params = useParams();
  const [req] = createResource<ChatBindRequest>(() =>
    api.getChatBindRequest(params.id)
  );
  const [groups] = createResource(api.listGroups);
  const [groupUid, setGroupUid] = createSignal<string>("");
  const [submitting, setSubmitting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [success, setSuccess] = createSignal<string | null>(null);

  async function onConfirm(e: Event) {
    e.preventDefault();
    if (!req()) return;
    if (!groupUid()) {
      setError("Please select an expense group");
      return;
    }
    setSubmitting(true);
    setError(null);
    setSuccess(null);
    try {
      const user = auth.user();
      if (!user) throw new Error("Not signed in");
      const created = await api.createChatBinding({
        group_uid: groupUid(),
        nonce: req()!.nonce,
        request_id: req()!.id,
      });
      setSuccess(`Binding created: ${created.id}`);
    } catch (err: any) {
      setError(err.message || "Failed to create binding");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div class="card">
      <h2 class="text-xl font-semibold mb-4">Confirm Chat Binding</h2>
      <Show when={req.loading || groups.loading}>
        <div>Loading...</div>
      </Show>
      <Show when={error()}>
        <div class="mb-3 text-red-600">{error()}</div>
      </Show>
      <Show when={success()}>
        <div class="mb-3 text-green-600">{success()}</div>
      </Show>
      <Show when={req()}>
        <div class="mb-4">
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 text-sm">
            <div>
              <span class="font-medium">Platform:</span> {req()!.platform}
            </div>
            <div>
              <span class="font-medium">Chat ID:</span> {req()!.p_uid}
            </div>
            <div>
              <span class="font-medium">Nonce:</span> {req()!.nonce}
            </div>
            <div>
              <span class="font-medium">Expires:</span>{" "}
              {new Date(req()!.expires_at).toLocaleString()}
            </div>
          </div>
        </div>
        <form onSubmit={onConfirm} class="space-y-4">
          <div>
            <label class="label" for="group">
              Select Expense Group
            </label>
            <select
              id="group"
              class="input"
              value={groupUid()}
              onInput={(e) => setGroupUid(e.currentTarget.value)}
              required
            >
              <option value="" disabled>
                Select a group
              </option>
              <For each={groups() || []}>
                {(g: ExpenseGroup) => <option value={g.uid}>{g.name}</option>}
              </For>
            </select>
          </div>
          <button
            class="btn"
            type="submit"
            disabled={submitting() || !groupUid()}
          >
            {submitting() ? "Confirming..." : "Confirm Binding"}
          </button>
        </form>
      </Show>
    </div>
  );
}
