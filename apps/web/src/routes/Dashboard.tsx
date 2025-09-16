import { createResource, For, Show } from 'solid-js';
import { api, type ExpenseGroup } from '../lib/api';

export default function Dashboard() {
  const [groups] = createResource(api.listGroups);

  return (
    <div class="space-y-4">
      <div class="card">
        <h2 class="text-xl font-semibold mb-2">Expense Groups</h2>
        <Show when={groups.loading}>
          <div>Loading...</div>
        </Show>
        <Show when={groups.error}>
          <div class="text-red-600">Failed to load groups.</div>
        </Show>
        <ul class="divide-y">
          <For each={groups() || []}>
            {(g: ExpenseGroup) => (
              <li class="py-2 flex items-center justify-between">
                <div>
                  <div class="font-medium">{g.name}</div>
                  <div class="text-xs text-gray-500">{g.uid}</div>
                </div>
                <div class="text-xs text-gray-500">Created {new Date(g.created_at).toLocaleString()}</div>
              </li>
            )}
          </For>
        </ul>
      </div>
    </div>
  );
}

