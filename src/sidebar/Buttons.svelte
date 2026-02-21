<script lang="ts">
  import MdSettings from "svelte-icons/md/MdSettings.svelte";
  import SettingsPopup from "./SettingsPopup.svelte";
  import IoMdTrash from "svelte-icons/io/IoMdTrash.svelte";
  import FaPuzzlePiece from "svelte-icons/fa/FaPuzzlePiece.svelte";
  import GiNuclear from "svelte-icons/gi/GiNuclear.svelte";
  import RebootPopup from "./RebootPopup.svelte";
  import FaTerminal from "svelte-icons/fa/FaTerminal.svelte";
  import TerminalPopup from "./TerminalPopup.svelte";
  import PluginPanel from "./PluginPanel.svelte";
  import { getContext } from "svelte";
  import { GLOBAL_FUNCTIONS } from "../context";

  const { refreshList } = getContext<{ refreshList: () => void }>(GLOBAL_FUNCTIONS);

  let bSetting: boolean;
  let bReboot: boolean;
  let bTerminal: boolean;
  let bPlugin: boolean;

  function toggleSettings() {
    bSetting = !bSetting;
  }
  function toggleReboot() {
    bReboot = !bReboot;
  }
  function toggleTerminal() {
    bTerminal = !bTerminal;
  }
  function togglePlugin() {
    bPlugin = !bPlugin;
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

  <button class="p-2" on:click={togglePlugin}>
    <div class="w-6 h-6">
      <FaPuzzlePiece />
    </div>
  </button>

  <button class="p-2 opacity-30 cursor-not-allowed" disabled>
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

<SettingsPopup show={bSetting} closeSettings={toggleSettings} onServerSwitch={refreshList} />
<RebootPopup show={bReboot} closeReboot={toggleReboot} />
<TerminalPopup show={bTerminal} closeTerminal={toggleTerminal} />
<PluginPanel show={bPlugin} closePlugin={togglePlugin} />
