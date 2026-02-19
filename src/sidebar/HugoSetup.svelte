<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { AppConfig, PrerequisiteResult } from "../types/setting";

  export let config: AppConfig;
  export let isSetupRunning: boolean = false;

  type StepStatus = "pending" | "running" | "done" | "error" | "warn";
  interface Step {
    label: string;
    status: StepStatus;
    message: string;
  }

  type Mode = "idle" | "new_site_form" | "new_site_running";

  let mode: Mode = "idle";
  let steps: Step[] = [];
  let errorMessage: string = "";
  let successMessage: string = "";

  // New Site form inputs
  let hugoVersion: string = "latest";
  let themeUrl: string = "https://github.com/theNewDynamic/gohugo-theme-ananke.git";

  function resetState() {
    mode = "idle";
    steps = [];
    errorMessage = "";
    successMessage = "";
    hugoVersion = "latest";
    themeUrl = "https://github.com/theNewDynamic/gohugo-theme-ananke.git";
    isSetupRunning = false;
  }

  function initSteps(labels: string[]): Step[] {
    return labels.map((label) => ({ label, status: "pending" as StepStatus, message: "" }));
  }

  function updateStep(index: number, status: StepStatus, message: string = "") {
    steps[index].status = status;
    steps[index].message = message;
    steps = steps;
  }

  function statusIcon(status: StepStatus): string {
    switch (status) {
      case "done": return "\u2713";
      case "running": return "\u23F3";
      case "pending": return "\u25CB";
      case "error": return "\u2715";
      case "warn": return "\u26A0";
    }
  }

  async function checkSshConnection(): Promise<boolean> {
    try {
      await invoke("execute_ssh", { cmd: "echo ok" });
      return true;
    } catch {
      return false;
    }
  }

  // ── New Site: show form ──
  async function showNewSiteForm() {
    errorMessage = "";
    if (!(await checkSshConnection())) {
      errorMessage = "SSH session not connected. Save SSH settings first.";
      return;
    }
    mode = "new_site_form";
  }

  // ── New Site: run all steps ──
  async function runNewSiteSetup() {
    mode = "new_site_running";
    isSetupRunning = true;
    errorMessage = "";

    steps = initSteps([
      "Check prerequisites",
      "Check / Install Hugo",
      "Create site & git init",
      "Install theme",
      "Configure paths",
    ]);

    try {
      // Step 0: prerequisites
      updateStep(0, "running");
      const prereq: PrerequisiteResult = await invoke("check_prerequisites_cmd");
      if (!prereq.curl || !prereq.tar) {
        updateStep(0, "error", `Missing: ${!prereq.curl ? "curl " : ""}${!prereq.tar ? "tar" : ""}`);
        errorMessage = "curl and tar are required. Please install them on the server.";
        return;
      }
      if (!prereq.git) {
        updateStep(0, "error", "git not found");
        errorMessage = "git is required for theme installation. Please install git on the server.";
        return;
      }
      updateStep(0, "done");

      // Step 1: check / install hugo
      updateStep(1, "running");
      let hugoPath: string | null = await invoke("check_hugo_installed_cmd");
      if (!hugoPath) {
        updateStep(1, "running", "Installing Hugo...");
        const [os, arch]: [string, string] = await invoke("detect_server_platform_cmd");
        let version = hugoVersion.trim();
        if (version === "latest" || version === "") {
          version = await invoke("get_latest_hugo_version_cmd");
        }
        hugoPath = await invoke("install_hugo_cmd", { os, arch, version });
      }
      updateStep(1, "done", hugoPath!);

      // Step 2: create site & git init
      updateStep(2, "running");
      const [siteName, sitePath]: [string, string] = await invoke("generate_site_name_cmd");
      await invoke("create_hugo_site_cmd", { hugoCmdPath: hugoPath!, sitePath });
      await invoke("git_init_site_cmd", { sitePath });
      updateStep(2, "done", `${siteName} (${sitePath})`);

      // Step 3: install theme
      updateStep(3, "running");
      if (themeUrl.trim()) {
        const themeName: string = await invoke("install_theme_cmd", {
          themeUrl: themeUrl.trim(),
          sitePath,
        });
        updateStep(3, "done", themeName);
      } else {
        updateStep(3, "warn", "Skipped (no URL)");
      }

      // Step 4: fill config
      updateStep(4, "running");
      config.cms_config.hugo_config.hugo_cmd_path = hugoPath!;
      config.cms_config.hugo_config.base_path = sitePath;
      config.cms_config.hugo_config.content_path = "posts";
      config.cms_config.hugo_config.image_path = "static";
      config = config;
      updateStep(4, "done", "Config fields populated");

    } catch (e: any) {
      const failIdx = steps.findIndex((s) => s.status === "running");
      if (failIdx >= 0) updateStep(failIdx, "error", String(e));
      errorMessage = String(e);
    } finally {
      isSetupRunning = false;
    }
  }

  // ── Connect Existing Flow ──
  async function connectExisting() {
    errorMessage = "";
    successMessage = "";

    try {
      // 1. Save current config (saves SSH settings + connects SSH)
      await invoke("save_config", { config });

      // 2. Reload config (loads server config from ~/.inn_server_config.json)
      const loaded: AppConfig = await invoke("load_config");
      config = { ...config, cms_config: loaded.cms_config };
      config = config;

      // 3. Validate loaded server config
      const hugo = config.cms_config.hugo_config;
      if (!hugo.base_path || !hugo.content_path) {
        errorMessage = "Server config not found. ~/.inn_server_config.json is missing or incomplete.";
        return;
      }

      // 4. Validate Hugo project exists at base_path
      const isValid: boolean = await invoke("validate_hugo_project_cmd", { path: hugo.base_path });
      if (!isValid) {
        errorMessage = `Not a valid Hugo project at ${hugo.base_path}.`;
        return;
      }

      successMessage = "Connected successfully. Click Save and Exit.";
    } catch (e: any) {
      errorMessage = String(e);
    }
  }
</script>

<div class="setup-container">
  {#if mode === "idle"}
    <div class="setup-title">Quick Setup</div>
    <div class="button-row">
      <button class="setup-btn" on:click={showNewSiteForm}>New Site</button>
      <button class="setup-btn" on:click={connectExisting}>Connect Existing</button>
    </div>
    {#if successMessage}
      <div class="success-box">{successMessage}</div>
    {/if}
    {#if errorMessage}
      <div class="error-box">{errorMessage}</div>
    {/if}

  {:else if mode === "new_site_form"}
    <div class="setup-title">New Site Options</div>
    <div class="form-section">
      <label class="input-label" for="hugo-version-input">Hugo version:</label>
      <input
        id="hugo-version-input"
        type="text"
        class="form-input"
        bind:value={hugoVersion}
        placeholder="latest"
      />
    </div>
    <div class="form-section">
      <label class="input-label" for="theme-url-input">Theme git URL (empty to skip):</label>
      <input
        id="theme-url-input"
        type="text"
        class="form-input"
        bind:value={themeUrl}
      />
    </div>
    <div class="button-row">
      <button class="setup-btn" on:click={runNewSiteSetup}>Start Setup</button>
      <button class="setup-btn" on:click={resetState}>Cancel</button>
    </div>

  {:else}
    <!-- Step list -->
    <div class="steps-list">
      {#each steps as step, i}
        <div class="step-row" class:step-done={step.status === "done"} class:step-error={step.status === "error"} class:step-warn={step.status === "warn"} class:step-running={step.status === "running"}>
          <span class="step-icon">{statusIcon(step.status)}</span>
          <span class="step-label">{step.label}</span>
          {#if step.message}
            <span class="step-message">- {step.message}</span>
          {/if}
        </div>
      {/each}
    </div>

    <!-- Error message + retry -->
    {#if errorMessage}
      <div class="error-box">{errorMessage}</div>
    {/if}

    <button class="retry-btn" on:click={resetState}>
      {errorMessage ? "Retry" : "Back"}
    </button>
  {/if}
</div>

<style>
  .setup-container {
    margin-bottom: 1rem;
    padding: 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 0.5rem;
  }

  .setup-title {
    font-weight: bold;
    margin-bottom: 0.5rem;
    font-size: 0.9rem;
  }

  .button-row {
    display: flex;
    gap: 0.5rem;
  }

  .setup-btn {
    flex: 1;
    padding: 0.4rem 0.8rem;
    font-size: 0.85rem;
  }

  .form-section {
    margin-bottom: 0.5rem;
  }

  .form-input {
    width: 100%;
    padding: 0.4rem 0.6rem;
    font-size: 0.85rem;
    box-sizing: border-box;
  }

  .input-label {
    font-size: 0.8rem;
    margin-bottom: 0.25rem;
    display: block;
    opacity: 0.8;
  }

  .steps-list {
    margin-bottom: 0.5rem;
  }

  .step-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.2rem 0;
    font-size: 0.85rem;
  }

  .step-icon {
    width: 1.2rem;
    text-align: center;
    flex-shrink: 0;
  }

  .step-done .step-icon { color: var(--success-color); }
  .step-error .step-icon { color: var(--error-color); }
  .step-warn .step-icon { color: var(--warning-color); }
  .step-running .step-icon { color: var(--info-color); }

  .step-label {
    font-weight: 500;
  }

  .step-message {
    color: var(--reverse-secondary-color);
    opacity: 0.7;
    font-size: 0.8rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .success-box {
    color: var(--success-color);
    font-size: 0.8rem;
    padding: 0.4rem;
    margin: 0.5rem 0;
    border: 1px solid var(--success-color);
    border-radius: 0.25rem;
  }

  .error-box {
    color: var(--error-color);
    font-size: 0.8rem;
    padding: 0.4rem;
    margin: 0.5rem 0;
    border: 1px solid var(--error-color);
    border-radius: 0.25rem;
  }

  .retry-btn {
    width: 100%;
    padding: 0.4rem;
    font-size: 0.85rem;
    margin-top: 0.5rem;
  }
</style>
