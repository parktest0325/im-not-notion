<script lang="ts">
    import SettingsPopup from "./SettingsPopup.svelte";
        import RebootPopup from "./RebootPopup.svelte";
  import { Settings, SquareTerminal, FolderOpen, Puzzle, Power } from "lucide-svelte";
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
    <div class="w-6 h-6"><Settings size="100%" /></div>
  </button>

  <button class="p-2" on:click={toggleTerminal} title="Terminal">
    <div class="w-6 h-6"><SquareTerminal size="100%" /></div>
  </button>

  <button class="p-2" on:click={toggleExplorer} title="File Explorer">
    <div class="w-6 h-6"><FolderOpen size="100%" /></div>
  </button>

  <button class="p-2" on:click={togglePlugin} title="Plugins">
    <div class="w-6 h-6"><Puzzle size="100%" /></div>
  </button>

  <button class="p-2" on:click={toggleReboot} title="Reboot">
    <div class="w-6 h-6"><Power size="100%" /></div>
  </button>
</div>

<SettingsPopup show={bSetting} closeSettings={toggleSettings} onServerSwitch={refreshList} />
<RebootPopup show={bReboot} closeReboot={toggleReboot} />
<TerminalPopup show={bTerminal} closeTerminal={toggleTerminal} />
<FileExplorerPopup show={bExplorer} closeExplorer={toggleExplorer} />
<PluginPanel show={bPlugin} closePlugin={togglePlugin} />
