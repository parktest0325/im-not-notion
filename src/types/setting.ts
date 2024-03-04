interface SshClientConfig {
    host: string;
    port: string;
    username: string;
    password: string;
    key_path: string;
    [key: string]: string;
}

interface AppConfig {
    ssh_client: SshClientConfig;
}