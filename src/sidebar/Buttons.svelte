<script lang="ts">
  import MdSettings from "svelte-icons/md/MdSettings.svelte";
  import SettingsPopup from "./SettingsPopup.svelte";
  import FaFolderOpen from "svelte-icons/fa/FaFolderOpen.svelte";
  import FaPuzzlePiece from "svelte-icons/fa/FaPuzzlePiece.svelte";
  import GiNuclear from "svelte-icons/gi/GiNuclear.svelte";
  import RebootPopup from "./RebootPopup.svelte";
  import FaTerminal from "svelte-icons/fa/FaTerminal.svelte";
  import TerminalPopup from "./TerminalPopup.svelte";
  import PluginPanel from "./PluginPanel.svelte";
  import FileExplorerPopup from "./FileExplorerPopup.svelte";
  import { getContext } from "svelte";
  import { GLOBAL_FUNCTIONS } from "../context";

  const { refreshList } = getContext<{ refreshList: () => void }>(GLOBAL_FUNCTIONS);

  let bSetting: boolean;
  let bReboot: boolean;
  let bTerminal: boolean;
  let bPlugin: boolean;
  let bExplorer: boolean;

  function toggleSettings() { bSetting = !bSetting; }
  function toggleReboot() { bReboot = !bReboot; }
  function toggleTerminal() { bTerminal = !bTerminal; }
  function togglePlugin() { bPlugin = !bPlugin; }
  function toggleExplorer() { bExplorer = !bExplorer; }
</script>

<div class="flex justify-between max-w-4xl mx-auto">
  <button class="p-2" on:click={toggleSettings} title="Settings">
    <div class="w-6 h-6"><MdSettings /></div>
  </button>

  <button class="p-2" on:click={toggleTerminal} title="Terminal">
    <div class="w-6 h-6"><FaTerminal /></div>
  </button>

  <button class="p-2" on:click={toggleExplorer} title="File Explorer">
    <div class="w-6 h-6"><FaFolderOpen /></div>
  </button>

  <button class="p-2" on:click={togglePlugin} title="Plugins">
    <div class="w-6 h-6"><FaPuzzlePiece /></div>
  </button>

  <button class="p-2" on:click={toggleReboot} title="Reboot">
    <div class="w-6 h-6"><GiNuclear /></div>
  </button>
</div>

<SettingsPopup show={bSetting} closeSettings={toggleSettings} onServerSwitch={refreshList} />
<RebootPopup show={bReboot} closeReboot={toggleReboot} />
<TerminalPopup show={bTerminal} closeTerminal={toggleTerminal} />
<FileExplorerPopup show={bExplorer} closeExplorer={toggleExplorer} />
<PluginPanel show={bPlugin} closePlugin={togglePlugin} />
