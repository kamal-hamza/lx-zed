use std::fs;
use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree};

struct LxExtension {
    cached_binary_path: Option<String>,
}

impl LxExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    /// Get the binary filename based on the platform
    fn get_binary_name(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        return "lx-lsp.exe";

        #[cfg(not(target_os = "windows"))]
        return "lx-lsp";
    }

    /// Check if Go is available
    fn check_go_available(&self) -> bool {
        let result = zed::Command::new("go")
            .args(vec!["version".to_string()])
            .output();

        if let Ok(output) = result {
            eprintln!(
                "[LX Extension] go version: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            output.status == Some(0)
        } else {
            eprintln!("[LX Extension] go not found");
            false
        }
    }

    /// Install the language server using go install
    fn install_language_server(&self, language_server_id: &LanguageServerId) -> Result<String> {
        eprintln!("[LX Extension] Installing lx-lsp from github...");

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        // Install the language server using go install
        let install_result = zed::Command::new("go")
            .args(vec![
                "install".to_string(),
                "github.com/kamal-hamza/lx-lsp@latest".to_string(),
            ])
            .output();

        match install_result {
            Ok(output) => {
                eprintln!(
                    "[LX Extension] go install output: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
                eprintln!(
                    "[LX Extension] go install stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );

                if output.status != Some(0) {
                    let error_msg = format!(
                        "Failed to install lx-lsp. Error: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );

                    zed::set_language_server_installation_status(
                        language_server_id,
                        &zed::LanguageServerInstallationStatus::Failed(error_msg.clone()),
                    );

                    return Err(error_msg.into());
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to run go install: {:?}", e);
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::Failed(error_msg.clone()),
                );
                return Err(error_msg.into());
            }
        }

        // Use 'which' to find the installed binary
        eprintln!("[LX Extension] Locating installed binary with 'which'...");

        #[cfg(target_os = "windows")]
        let which_cmd = "where";
        #[cfg(not(target_os = "windows"))]
        let which_cmd = "which";

        let which_result = zed::Command::new(which_cmd)
            .args(vec![self.get_binary_name().to_string()])
            .output();

        let binary_path = match which_result {
            Ok(output) if output.status == Some(0) => {
                let path = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();

                if path.is_empty() {
                    return Err("Failed to locate installed language server binary".into());
                }

                path
            }
            _ => {
                // Fallback: Check default Go bin location ($HOME/go/bin)
                if let Ok(home) = std::env::var("HOME") {
                    let go_bin_path = format!("{}/go/bin/{}", home, self.get_binary_name());

                    if fs::metadata(&go_bin_path).is_ok() {
                        eprintln!(
                            "[LX Extension] Found binary in default GOPATH: {}",
                            go_bin_path
                        );
                        go_bin_path
                    } else {
                        return Err("Failed to locate installed language server binary".into());
                    }
                } else {
                    return Err("Failed to locate installed language server binary".into());
                }
            }
        };

        eprintln!(
            "[LX Extension] Language server installed at: {}",
            binary_path
        );

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::None,
        );

        Ok(binary_path)
    }

    /// Try to find an existing installation of the language server
    fn find_existing_binary(&self) -> Option<String> {
        eprintln!("[LX Extension] Searching for existing binary...");

        // Use 'which' to find the binary in PATH
        #[cfg(target_os = "windows")]
        let which_cmd = "where";
        #[cfg(not(target_os = "windows"))]
        let which_cmd = "which";

        let which_result = zed::Command::new(which_cmd)
            .args(vec![self.get_binary_name().to_string()])
            .output();

        if let Ok(output) = which_result {
            if output.status == Some(0) {
                let path = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();

                if !path.is_empty() {
                    eprintln!("[LX Extension] Found binary in PATH: {}", path);
                    return Some(path);
                }
            }
        }

        // Fallback: Check default Go bin location ($HOME/go/bin)
        if let Ok(home) = std::env::var("HOME") {
            let go_bin_path = format!("{}/go/bin/{}", home, self.get_binary_name());

            if fs::metadata(&go_bin_path).is_ok() {
                eprintln!(
                    "[LX Extension] Found binary in default GOPATH: {}",
                    go_bin_path
                );
                return Some(go_bin_path);
            }
        }

        eprintln!("[LX Extension] No existing binary found");
        None
    }

    /// Main function to get or install the language server binary
    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        _worktree: &Worktree,
    ) -> Result<String> {
        eprintln!("[LX Extension] ========== Starting language_server_binary_path ==========");

        // Check cache first
        if let Some(path) = &self.cached_binary_path {
            eprintln!("[LX Extension] ✓ Using cached binary path: {}", path);
            return Ok(path.clone());
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        // Try to find existing binary
        if let Some(existing_path) = self.find_existing_binary() {
            eprintln!("[LX Extension] ✓ Using existing binary: {}", existing_path);
            self.cached_binary_path = Some(existing_path.clone());

            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );

            return Ok(existing_path);
        }

        // Check if Go is available
        if !self.check_go_available() {
            let error_msg = "Go toolchain not found. Please install Go to use LX extension.\n\n\
                 Install Go from https://go.dev/doc/install\n\n\
                 The extension will provide syntax highlighting only.";

            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Failed(error_msg.to_string()),
            );

            return Err(error_msg.into());
        }

        // Install the language server
        let binary_path = self.install_language_server(language_server_id)?;

        // Cache the path
        self.cached_binary_path = Some(binary_path.clone());

        eprintln!("[LX Extension] ========== Finished successfully ==========");
        Ok(binary_path)
    }
}

impl zed::Extension for LxExtension {
    fn new() -> Self {
        Self::new()
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let binary_path = self.language_server_binary_path(language_server_id, worktree)?;

        eprintln!("[LX Extension] Starting language server: {}", binary_path);

        Ok(zed::Command {
            command: binary_path,
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(LxExtension);
