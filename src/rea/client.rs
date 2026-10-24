use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

/// Cliente para comunicarse con las herramientas de REA.
#[derive(Debug, Clone, Default)]
pub struct ReaClient;

impl ReaClient {
    pub fn new() -> Self {
        Self
    }

    fn run_rea_command(&self, subcmd: &str, args: &[&str]) -> Result<String> {
        let launcher = if cfg!(windows) { "npx.cmd" } else { "npx" };
        let mut cmd = Command::new(launcher);
        cmd.args(["-y", "rea-agents@latest", subcmd]);
        cmd.args(args);

        let output = cmd
            .output()
            .with_context(|| format!("Error ejecutando comando REA '{subcmd}'"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = if !stderr.trim().is_empty() {
                stderr.trim()
            } else {
                stdout.trim()
            };
            bail!(
                "Comando REA '{subcmd}' falló con código {:?}: {detail}",
                output.status.code()
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// REA actual usa `inspect`; puede requerir un proveedor profundo.
    pub fn inspect_pe_imports(&self, target_exe: &Path) -> Result<serde_json::Value> {
        crate::local_pe::inspect_imports(target_exe)
    }

    /// Inspecciona recursos PE. El límite se ajusta al tamaño real del artefacto.
    pub fn inspect_pe_resources(&self, target_exe: &Path) -> Result<serde_json::Value> {
        let path_str = target_exe.to_string_lossy();
        let max_file_bytes = target_exe
            .metadata()
            .map(|metadata| metadata.len().saturating_add(1024 * 1024).to_string())
            .unwrap_or_else(|_| "536870912".to_string());
        let raw = self.run_rea_command(
            "inspect-pe-resources",
            &[
                &path_str,
                "--max-file-bytes",
                &max_file_bytes,
                "--format",
                "json",
            ],
        )?;
        serde_json::from_str(&raw).context("Error parseando salida JSON de inspect-pe-resources")
    }

    /// Authenticode se consulta localmente en Windows; no requiere un proveedor REA.
    pub fn inspect_pe_certificates(&self, target_exe: &Path) -> Result<serde_json::Value> {
        crate::local_pe::inspect_authenticode(target_exe)
    }

    pub fn inspect_artifact(&self, target_exe: &Path) -> Result<serde_json::Value> {
        let path_str = target_exe.to_string_lossy();
        let raw = self.run_rea_command("inspect-artifact", &[&path_str, "--format", "json"])?;
        serde_json::from_str(&raw).context("Error parseando salida JSON de inspect-artifact")
    }

    pub fn search_procedures(
        &self,
        target_exe: &Path,
        pattern: &str,
        provider: &str,
    ) -> Result<serde_json::Value> {
        let path_str = target_exe.to_string_lossy();
        let raw = self.run_rea_command(
            "search",
            &[
                &path_str,
                pattern,
                "--kind",
                "procedures",
                "--provider",
                provider,
                "--format",
                "json",
            ],
        )?;
        serde_json::from_str(&raw).context("Error parseando salida JSON de search procedures")
    }

    pub fn decompile(&self, target_exe: &Path, procedure: &str, provider: &str) -> Result<String> {
        let path_str = target_exe.to_string_lossy();
        let proc_flag = format!("--procedure={procedure}");
        self.run_rea_command(
            "decompile",
            &[&path_str, &proc_flag, "--provider", provider],
        )
    }
}
