<!-- Popup.svelte -->
<script lang="ts">
  export let show: boolean;
  export let isLoading: boolean = false;
  export let closePopup: () => void;
</script>

{#if show}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div
    class="fixed inset-0 flex justify-center items-center p-4 popup-overlay"
    on:click|self={closePopup}
  >
    {#if isLoading}
      <p>Loading...</p>
    {:else}
      <div class="popup-content">
        <slot />
      </div>
    {/if}
  </div>
{/if}

<style>
  .popup-overlay {
    background-color: var(--overlay-bg-color);
    z-index: 1000;
  }

  .popup-content {
    background-color: var(--popup-bg-color);
    color: var(--popup-text-color);
    padding: 1.5rem;
    border-radius: 0.5rem;
    box-shadow: var(--shadow-popup);
    width: 100%;
    max-width: 32rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
</style>
