<script lang="ts">
  import MdSettings from "svelte-icons/md/MdSettings.svelte";
  import SettingsPopup from "./SettingsPopup.svelte";
  import IoMdTrash from "svelte-icons/io/IoMdTrash.svelte";
  import FaServer from "svelte-icons/fa/FaServer.svelte";
  import GiNuclear from "svelte-icons/gi/GiNuclear.svelte";
  import RebootPopup from "./RebootPopup.svelte";
  import FaTerminal from "svelte-icons/fa/FaTerminal.svelte";
  import TerminalPopup from "./TerminalPopup.svelte";

  let bSetting: boolean;
  let bReboot: boolean;
  let bTerminal: boolean;

  function toggleSettings() {
    bSetting = !bSetting;
  }
  function toggleReboot() {
    bReboot = !bReboot;
  }
  function toggleTerminal() {
    bTerminal = !bTerminal;
  }
</script>

<div class="flex justify-between max-w-4xl mx-auto">
  <button class="p-2" on:click={toggleSettings}>
    <div class="w-6 h-6">
      <MdSettings />
    </div>
  </button>

  <button class="p-2" on:click={toggleTerminal}>
    <div class="w-6 h-6">
      <FaTerminal />
    </div>
  </button>

  <button class="p-2" on:click={toggleSettings}>
    <div class="w-6 h-6">
      <FaServer />
    </div>
  </button>

  <button class="p-2" on:click={toggleSettings}>
    <div class="w-6 h-6">
      <IoMdTrash />
    </div>
  </button>

  <button class="p-2" on:click={toggleReboot}>
    <div class="w-6 h-6">
      <GiNuclear />
    </div>
  </button>
</div>

<SettingsPopup show={bSetting} closeSettings={toggleSettings} />
<RebootPopup show={bReboot} closeReboot={toggleReboot} />
<TerminalPopup show={bTerminal} closeTerminal={toggleTerminal} />
