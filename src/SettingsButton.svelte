<script lang="ts">
  import MdSettings from "svelte-icons/md/MdSettings.svelte";
  import SettingsPopup from "./SettingsPopup.svelte";
  let show: boolean;

  function toggleSettings() {
    show = !show;
  }
</script>

<button on:click={toggleSettings}>
  <MdSettings />
</button>

<SettingsPopup {show} closeSettings={toggleSettings} />

<style>
  button {
    /* 버튼 스타일 */
  }
</style>
