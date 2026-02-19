use std::path::Path;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use crate::services::ssh_service::{get_channel_session, get_sftp_session, execute_ssh_command};
use crate::services::config_service::get_server_home_path;

/// channel 생성 + 명령 실행을 한 줄로 처리
fn run_ssh(cmd: &str) -> Result<String> {
    let mut channel = get_channel_session()?;
    execute_ssh_command(&mut channel, cmd)
}

const GREEK_NAMES: [&str; 24] = [
    "alpha", "beta", "gamma", "delta", "epsilon", "zeta",
    "eta", "theta", "iota", "kappa", "lambda", "mu",
    "nu", "xi", "omicron", "pi", "rho", "sigma",
    "tau", "upsilon", "phi", "chi", "psi", "omega",
];

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrerequisiteResult {
    pub curl: bool,
    pub tar: bool,
    pub git: bool,
}

/// Check if curl, tar, git are available on the server
pub fn check_prerequisites() -> Result<PrerequisiteResult> {
    let curl_out = run_ssh("which curl")?;
    let tar_out = run_ssh("which tar")?;
    let git_out = run_ssh("which git")?;

    Ok(PrerequisiteResult {
        curl: !curl_out.trim().is_empty(),
        tar: !tar_out.trim().is_empty(),
        git: !git_out.trim().is_empty(),
    })
}

/// Check if hugo is installed on the server
/// Returns the path to hugo if found, None otherwise
pub fn check_hugo_installed() -> Result<Option<String>> {
    // Try `which hugo` first
    let which_out = run_ssh("which hugo")?;
    let which_trimmed = which_out.trim();
    if !which_trimmed.is_empty() && !which_trimmed.contains("not found") {
        return Ok(Some(which_trimmed.to_string()));
    }

    // Try ~/.local/bin/hugo
    let test_out = run_ssh("test -x ~/.local/bin/hugo && echo 'exists'")?;
    if test_out.trim() == "exists" {
        let home = get_server_home_path()?;
        return Ok(Some(format!("{}/.local/bin/hugo", home)));
    }

    Ok(None)
}

/// Detect server OS and architecture
/// Returns (os, arch) tuple e.g. ("Linux", "amd64")
pub fn detect_server_platform() -> Result<(String, String)> {
    let os = run_ssh("uname -s")?;
    let os = os.trim().to_string();

    let arch_raw = run_ssh("uname -m")?;
    let arch_raw = arch_raw.trim();

    let arch = match arch_raw {
        "x86_64" => "amd64".to_string(),
        "aarch64" | "arm64" => "arm64".to_string(),
        other => other.to_string(),
    };

    Ok((os, arch))
}

/// Get the latest Hugo version from GitHub API
pub fn get_latest_hugo_version() -> Result<String> {
    let output = run_ssh(
        "curl -sL https://api.github.com/repos/gohugoio/hugo/releases/latest | grep '\"tag_name\"' | head -1 | sed 's/.*\"v\\([^\"]*\\)\".*/\\1/'"
    )?;

    let version = output.trim().to_string();
    if version.is_empty() {
        bail!("Failed to get latest Hugo version from GitHub API");
    }

    Ok(version)
}

/// Install Hugo on the server
/// Returns the absolute path to the installed hugo binary
pub fn install_hugo(os: &str, arch: &str, version: &str) -> Result<String> {
    run_ssh("mkdir -p ~/.local/bin")?;

    // Build download URL
    let platform = if os == "Darwin" {
        "darwin-universal".to_string()
    } else {
        format!("{}-{}", os.to_lowercase(), arch)
    };
    let url = format!(
        "https://github.com/gohugoio/hugo/releases/download/v{}/hugo_extended_{}_{}.tar.gz",
        version, version, platform
    );

    run_ssh(&format!("curl -sL '{}' -o /tmp/hugo_extended.tar.gz", url))?;
    run_ssh("tar -xzf /tmp/hugo_extended.tar.gz -C ~/.local/bin/ hugo")?;
    run_ssh("rm -f /tmp/hugo_extended.tar.gz")?;
    run_ssh("chmod +x ~/.local/bin/hugo")?;

    // Verify
    let verify = run_ssh("~/.local/bin/hugo version")?;
    if verify.trim().is_empty() {
        bail!("Hugo installation verification failed");
    }

    let home = get_server_home_path()?;
    Ok(format!("{}/.local/bin/hugo", home))
}

/// Generate a unique site name using Greek alphabet
/// Returns (name, full_path) tuple
pub fn generate_site_name() -> Result<(String, String)> {
    let home = get_server_home_path()?;

    let sftp = get_sftp_session()?;

    // Try base Greek names first
    for name in &GREEK_NAMES {
        let full_path = format!("{}/{}", home, name);
        if sftp.stat(Path::new(&full_path)).is_err() {
            return Ok((name.to_string(), full_path));
        }
    }

    // Try Greek names with suffix -1, -2, ...
    for suffix in 1..100 {
        for name in &GREEK_NAMES {
            let suffixed = format!("{}-{}", name, suffix);
            let full_path = format!("{}/{}", home, suffixed);
            if sftp.stat(Path::new(&full_path)).is_err() {
                return Ok((suffixed, full_path));
            }
        }
    }

    bail!("Could not generate a unique site name")
}

/// Create a new Hugo site at the given path
pub fn create_hugo_site(hugo_cmd_path: &str, site_path: &str) -> Result<()> {
    run_ssh(&format!("{} new site {}", hugo_cmd_path, site_path))?;
    // Verify site was created
    let sftp = get_sftp_session()?;
    if sftp.stat(Path::new(site_path)).is_err() {
        bail!("Hugo site creation failed: directory not found at {}", site_path);
    }

    Ok(())
}

/// Initialize git repo in the site directory
pub fn git_init_site(site_path: &str) -> Result<()> {
    run_ssh(&format!("cd {} && git init", site_path))?;
    Ok(())
}

/// Install a Hugo theme via git submodule
/// theme_url: git repo URL (e.g. "https://github.com/adityatelange/hugo-PaperMod.git")
/// site_path: Hugo site root path
/// Returns the theme directory name
pub fn install_theme(theme_url: &str, site_path: &str) -> Result<String> {
    // Extract theme name from URL: "https://github.com/user/hugo-PaperMod.git" -> "hugo-PaperMod"
    let theme_name = theme_url
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .rsplit('/')
        .next()
        .unwrap_or("theme")
        .to_string();

    // git submodule add
    run_ssh(&format!(
        "cd {} && git submodule add {} themes/{}",
        site_path, theme_url, theme_name
    ))?;
    // Verify theme directory exists
    let sftp = get_sftp_session()?;
    let theme_path = format!("{}/themes/{}", site_path, theme_name);
    if sftp.stat(Path::new(&theme_path)).is_err() {
        bail!("Theme installation failed: {} not found", theme_path);
    }

    // Append theme to hugo.toml
    let config_path = format!("{}/hugo.toml", site_path);
    run_ssh(&format!("printf '\\ntheme = \"{}\"\\n' >> {}", theme_name, config_path))?;

    Ok(theme_name)
}

/// Validate that a path contains a Hugo project
/// Checks for hugo.toml, config.toml, hugo.yaml, config.yaml, hugo.json, config.json
pub fn validate_hugo_project(path: &str) -> Result<bool> {
    let sftp = get_sftp_session()?;
    let config_files = [
        "hugo.toml", "config.toml",
        "hugo.yaml", "config.yaml",
        "hugo.json", "config.json",
    ];

    for config_file in &config_files {
        let full_path = format!("{}/{}", path, config_file);
        if sftp.stat(Path::new(&full_path)).is_ok() {
            return Ok(true);
        }
    }

    Ok(false)
}
