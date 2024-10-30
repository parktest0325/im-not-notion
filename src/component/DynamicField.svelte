<script lang="ts">
    export let config: Record<string, string>;
    export let configKey: string;
    export let hidePassword: boolean = true;
  
    let isPasswordField = configKey.toLowerCase().includes("password");
  
    function toggleHidePassword() {
      hidePassword = !hidePassword;
    }
  </script>
  
  <div class="flex items-center space-x-2">
    <!-- svelte-ignore a11y-label-has-associated-control -->
    <label class="block min-w-[120px]">{configKey}</label>
    {#if isPasswordField && hidePassword}
      <input
        class="flex-1 p-2 border rounded"
        type="password"
        bind:value={config[configKey]}
      />
    {:else}
      <input class="flex-1 p-2 border rounded" bind:value={config[configKey]} />
    {/if}
    {#if isPasswordField}
      <button class="p-2 border rounded" on:click={toggleHidePassword}>
        {hidePassword ? "Show" : "Hide"}
      </button>
    {/if}
  </div>