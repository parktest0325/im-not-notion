<script lang="ts">
  import MdSettings from "svelte-icons/md/MdSettings.svelte";
  import SettingsPopup from "./SettingsPopup.svelte";
  import FaFileCode from "svelte-icons/fa/FaFileCode.svelte";
  import IoMdTrash from "svelte-icons/io/IoMdTrash.svelte";
  import FaHome from "svelte-icons/fa/FaHome.svelte";

  let show: boolean;

  function toggleSettings() {
    show = !show;
  }
</script>

<button class="p-2 mr-4" on:click={toggleSettings}>
  <div class="w-6 h-6">
    <MdSettings />
  </div>
</button>

<button class="p-2 mr-4" on:click={toggleSettings}>
  <div class="w-6 h-6">
    <FaFileCode />
  </div>
</button>

<button class="p-2 mr-4" on:click={toggleSettings}>
  <div class="w-6 h-6">
    <FaHome />
  </div>
</button>

<button class="p-2" on:click={toggleSettings}>
  <div class="w-6 h-6">
    <IoMdTrash />
  </div>
</button>

<SettingsPopup {show} closeSettings={toggleSettings} />
