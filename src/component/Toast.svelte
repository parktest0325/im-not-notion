<script lang="ts">
  import { fly } from "svelte/transition";
  import { toasts } from "../stores";

  function dismiss(id: number) {
    toasts.update(t => t.filter(item => item.id !== id));
  }
</script>

<div class="toast-container">
  {#each $toasts as toast (toast.id)}
    <button
      class="toast toast-{toast.type}"
      transition:fly={{ x: 300, duration: 300 }}
      on:click={() => dismiss(toast.id)}
    >
      {toast.message}
    </button>
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
    padding: 0.75rem 1rem;
    border-radius: 0.5rem;
    font-size: 0.85rem;
    text-align: left;
    cursor: pointer;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    border: none;
    width: 100%;
  }

  .toast-error {
    background: var(--error-color);
    color: #fff;
  }

  .toast-success {
    background: var(--success-color);
    color: #fff;
  }

  .toast-info {
    background: var(--info-color);
    color: #fff;
  }
</style>
