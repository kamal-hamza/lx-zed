use std::fs;
use std::process::Command;
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
        // FIX: Use std::process::Command instead of zed::Command
        let result = Command::new("go").arg("version").output();

        if let Ok(output) = result {
            output.status.success()
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

        // FIX: Use std::process::Command
        let install_result = Command::new("go")
            .args(&["install", "github.com/kamal-hamza/lx-lsp@latest"])
            .output();

        match install_result {
            Ok(output) => {
                if !output.status.success() {
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
                eprintln!("[LX Extension] go install successful");
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

        self.find_installed_binary(language_server_id)
    }

    fn find_installed_binary(&self, language_server_id: &LanguageServerId) -> Result<String> {
        eprintln!("[LX Extension] Locating binary...");

        #[cfg(target_os = "windows")]
        let which_cmd = "where";
        #[cfg(not(target_os = "windows"))]
        let which_cmd = "which";

        // FIX: Use std::process::Command
        let which_result = Command::new(which_cmd).arg(self.get_binary_name()).output();

        if let Ok(output) = which_result {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();

                if !path.is_empty() {
                    eprintln!("[LX Extension] Found binary via which: {}", path);
                    zed::set_language_server_installation_status(
                        language_server_id,
                        &zed::LanguageServerInstallationStatus::None,
                    );
                    return Ok(path);
                }
            }
        }

        // Fallback: Check default Go bin location ($HOME/go/bin) explicitly
        if let Ok(home) = std::env::var("HOME") {
            let go_bin_path = format!("{}/go/bin/{}", home, self.get_binary_name());

            if fs::metadata(&go_bin_path).is_ok() {
                eprintln!(
                    "[LX Extension] Found binary in default GOPATH: {}",
                    go_bin_path
                );
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::None,
                );
                return Ok(go_bin_path);
            }
        }

        let err = "Could not find lx-lsp binary. Ensure $HOME/go/bin is in your PATH or go install succeeded.";
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Failed(err.to_string()),
        );
        Err(err.into())
    }

    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        _worktree: &Worktree,
    ) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            return Ok(path.clone());
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        if let Ok(path) = self.find_installed_binary(language_server_id) {
            self.cached_binary_path = Some(path.clone());
            return Ok(path);
        }

        if !self.check_go_available() {
            let error_msg = "Go toolchain not found. Please install Go to use LX extension.";
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Failed(error_msg.to_string()),
            );
            return Err(error_msg.into());
        }

        let binary_path = self.install_language_server(language_server_id)?;
        self.cached_binary_path = Some(binary_path.clone());
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

        // Here we use the zed::Command struct to tell Zed how to start the server
        Ok(zed::Command {
            command: binary_path,
            args: vec![],
            env: worktree.shell_env(),
        })
    }
}

zed::register_extension!(LxExtension);
