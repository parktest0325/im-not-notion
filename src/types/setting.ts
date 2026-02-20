/*
 * 타입 정의는 typeshare에 의해 generated.ts에서 자동 생성됩니다.
 * Rust 타입 변경 시: npm run typeshare
 */
export type {
    SshConfig,
    HugoConfig,
    CmsConfig,
    AppConfig,
    ServerEntry,
    FileSystemNode,
    PrerequisiteResult,
    PluginManifest,
    PluginInfo,
    Trigger,
    InputField,
    PluginResult,
    PluginAction,
} from "./generated";

export { NodeType, HookEvent } from "./generated";

import type { SshConfig, HugoConfig, CmsConfig, AppConfig, ServerEntry } from "./generated";

// 기본값이 포함된 객체 생성 함수
export function createDefaultSshConfig(): SshConfig {
    return { host: "", port: "", username: "", password: "" };
}

function createDefaultHugoConfig(): HugoConfig {
    return { url: "", hugo_cmd_path: "", base_path: "", content_path: "", image_path: "", hidden_path: "" };
}

function createDefaultCmsConfig(): CmsConfig {
    return { hugo_config: createDefaultHugoConfig() };
}

export function createDefaultAppConfig(): AppConfig {
    return { active_server: "", servers: [], cms_config: createDefaultCmsConfig() };
}

export function createDefaultServerEntry(): ServerEntry {
    return { id: "", name: "", ssh_config: createDefaultSshConfig() };
}
