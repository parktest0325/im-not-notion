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
    [key: string]: string;
}

interface AppConfig {
    ssh_config: SshConfig;
    hugo_config: HugoConfig;
}

interface FileSystemNode {
    name: string;
    type_: 'File' | 'Directory';
    children: FileSystemNode[];
}

function createDefaultAppConfig(): AppConfig {
    return {
        ssh_config: {
            host: 'None',
            port: 'None',
            username: 'None',
            password: 'None',
            key_path: 'None',
        },
        hugo_config: {
            hugo_cmd_path: 'None',
            base_path: 'None',
            content_path: 'None',
            image_path: 'None',
            config_path: 'None',
            layout_path: 'None',
        }
    };
}