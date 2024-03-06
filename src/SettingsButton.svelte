<script lang="ts">
  import MdSettings from "svelte-icons/md/MdSettings.svelte";
  import SettingsPopup from "./SettingsPopup.svelte";
  let show: boolean;

  function toggleSettings() {
    show = !show;
  }
</script>

<button class="p-2" on:click={toggleSettings}>
  <div class="w-6 h-6">
    <MdSettings />
  </div>
</button>

<SettingsPopup {show} closeSettings={toggleSettings} />
