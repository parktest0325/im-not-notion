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
