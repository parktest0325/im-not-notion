interface SshConfig {
    host: string;
    port: string;
    username: string;
    password: string;
    key_path: string;
    [key: string]: string;
}

interface HugoConfig {
    hugo_cmd_path: string,
    base_path: string,
    content_path: string,
    image_path: string,
    config_path: string,
    layout_path: string,
    trashcan_path: string,
    [key: string]: string;
}

interface CmsConfig {
    hugo_config: HugoConfig;
}

interface AppConfig {
    ssh_config: SshConfig;
    cms_config: CmsConfig;
}

interface FileSystemNode {
    name: string;
    type_: 'File' | 'Directory';
    children: FileSystemNode[];
}

// 기본값이 포함된 객체 생성 함수
function createDefaultSshConfig(): SshConfig {
    return {
        host: "",
        port: "",
        username: "",
        password: "",
        key_path: "",
    };
}

function createDefaultHugoConfig(): HugoConfig {
    return {
        hugo_cmd_path: "",
        base_path: "",
        content_path: "",
        image_path: "",
        config_path: "",
        layout_path: "",
        trashcan_path: "",
    };
}

function createDefaultCmsConfig(): CmsConfig {
    return {
        hugo_config: createDefaultHugoConfig(),
    };
}

function createDefaultAppConfig(): AppConfig {
    return {
        ssh_config: createDefaultSshConfig(),
        cms_config: createDefaultCmsConfig(),
    };
}
