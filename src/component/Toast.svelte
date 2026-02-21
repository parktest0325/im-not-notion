<script lang="ts">
  import { fly } from "svelte/transition";
  import { toasts } from "../stores";

  const DURATION = 3000;
  const timers = new Map<number, ReturnType<typeof setTimeout>>();

  function dismiss(id: number) {
    const t = timers.get(id);
    if (t) clearTimeout(t);
    timers.delete(id);
    toasts.update(t => t.filter(item => item.id !== id));
  }

  function startTimer(id: number) {
    const t = timers.get(id);
    if (t) clearTimeout(t);
    timers.set(id, setTimeout(() => dismiss(id), DURATION));
  }

  function pauseTimer(id: number) {
    const t = timers.get(id);
    if (t) clearTimeout(t);
  }

  $: $toasts.forEach(toast => {
    if (!timers.has(toast.id)) {
      startTimer(toast.id);
    }
  });
</script>

<div class="toast-container">
  {#each $toasts as toast (toast.id)}
    <div
      role="alert"
      class="toast toast-{toast.type}"
      transition:fly={{ x: 300, duration: 300 }}
      on:mouseenter={() => pauseTimer(toast.id)}
      on:mouseleave={() => startTimer(toast.id)}
    >
      <span class="toast-message">{toast.message}</span>
      <button class="toast-close" on:click={() => dismiss(toast.id)}>&times;</button>
    </div>
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    bottom: 1rem;
    right: 1rem;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-width: 24rem;
  }

  .toast {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-radius: 0.5rem;
    font-size: 0.85rem;
    text-align: left;
    box-shadow: var(--shadow-toast);
    width: 100%;
    user-select: text;
  }

  .toast-message {
    flex: 1;
    word-break: break-all;
  }

  .toast-close {
    flex-shrink: 0;
    background: none;
    border: none;
    color: inherit;
    font-size: 1.1rem;
    cursor: pointer;
    padding: 0;
    line-height: 1;
    opacity: 0.7;
    box-shadow: none;
  }

  .toast-close:hover {
    opacity: 1;
    background: none;
  }

  .toast-error {
    background: var(--error-color);
    color: var(--toast-text);
  }

  .toast-success {
    background: var(--success-color);
    color: var(--toast-text);
  }

  .toast-info {
    background: var(--info-color);
    color: var(--toast-text);
  }
</style>
