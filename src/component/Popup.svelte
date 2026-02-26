<!-- Popup.svelte -->
<script lang="ts">
  export let show: boolean;
  export let isLoading: boolean = false;
  export let closePopup: () => void;
  export let showCloseBtn: boolean = true;
</script>

{#if show}
  <div class="fixed inset-0 flex justify-center items-center p-4 popup-overlay">
    {#if isLoading}
      <p>Loading...</p>
    {:else}
      <div class="popup-content">
        {#if showCloseBtn}
          <button class="popup-close" title="Close" on:click={closePopup}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        {/if}
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
    position: relative;
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

  .popup-close {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 0.25rem;
    border: none;
    background: none;
    cursor: pointer;
    opacity: 0.3;
    z-index: 1;
  }
  .popup-close:hover {
    opacity: 0.8;
    background-color: var(--button-active-bg-color);
  }
</style>
