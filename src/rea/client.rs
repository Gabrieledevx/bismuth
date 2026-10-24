use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

/// Cliente para comunicarse con las herramientas de REA
#[derive(Debug, Clone, Default)]
pub struct ReaClient;

impl ReaClient {
    pub fn new() -> Self {
        Self
    }

    /// Ejecuta un comando de REA usando npx o comando directo
    fn run_rea_command(&self, subcmd: &str, args: &[&str]) -> Result<String> {
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/C", "npx", "-y", "rea-agents@latest", subcmd]);
            c
        } else {
            let mut c = Command::new("npx");
            c.args(["-y", "rea-agents@latest", subcmd]);
            c
        };

        for arg in args {
            cmd.arg(arg);
        }

        let output = cmd.output()
            .with_context(|| format!("Error ejecutando comando REA '{}'", subcmd))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Comando REA '{}' falló con código {:?}: {}", subcmd, output.status.code(), stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }

    /// Inspecciona la tabla de importaciones PE (APIs Win32 / DLLs requeridas)
    pub fn inspect_pe_imports(&self, target_exe: &Path) -> Result<serde_json::Value> {
        let path_str = target_exe.to_string_lossy();
        let raw = self.run_rea_command("inspect-pe-imports", &[&path_str, "--format", "json"])?;
        let json: serde_json::Value = serde_json::from_str(&raw)
            .with_context(|| "Error parseando salida JSON de inspect-pe-imports")?;
        Ok(json)
    }

    /// Inspecciona el árbol de recursos PE (Iconos, Manifiestos, Diálogos, Cadenas)
    pub fn inspect_pe_resources(&self, target_exe: &Path) -> Result<serde_json::Value> {
        let path_str = target_exe.to_string_lossy();
        let raw = self.run_rea_command("inspect-pe-resources", &[&path_str, "--format", "json"])?;
        let json: serde_json::Value = serde_json::from_str(&raw)
            .with_context(|| "Error parseando salida JSON de inspect-pe-resources")?;
        Ok(json)
    }

    /// Inspecciona los certificados digitales de Authenticode
    pub fn inspect_pe_certificates(&self, target_exe: &Path) -> Result<serde_json::Value> {
        let path_str = target_exe.to_string_lossy();
        let raw = self.run_rea_command("inspect-pe-certificates", &[&path_str, "--format", "json"])?;
        let json: serde_json::Value = serde_json::from_str(&raw)
            .with_context(|| "Error parseando salida JSON de inspect-pe-certificates")?;
        Ok(json)
    }

    /// Inspecciona si el binario es un contenedor/paquete (e.g. install4j, PyInstaller, Zip)
    pub fn inspect_artifact(&self, target_exe: &Path) -> Result<serde_json::Value> {
        let path_str = target_exe.to_string_lossy();
        let raw = self.run_rea_command("inspect-artifact", &[&path_str, "--detail", "summary", "--format", "json"])?;
        let json: serde_json::Value = serde_json::from_str(&raw)
            .with_context(|| "Error parseando salida JSON de inspect-artifact")?;
        Ok(json)
    }

    /// Busca procedimientos en el binario nativo (requiere proveedor como ghidra o kuna si está disponible)
    pub fn search_procedures(&self, target_exe: &Path, pattern: &str, provider: &str) -> Result<serde_json::Value> {
        let path_str = target_exe.to_string_lossy();
        let raw = self.run_rea_command("search", &[&path_str, pattern, "--kind", "procedures", "--provider", provider, "--format", "json"])?;
        let json: serde_json::Value = serde_json::from_str(&raw)
            .with_context(|| "Error parseando salida JSON de search procedures")?;
        Ok(json)
    }

    /// Descompila una función nativa a pseudocódigo C
    pub fn decompile(&self, target_exe: &Path, procedure: &str, provider: &str) -> Result<String> {
        let path_str = target_exe.to_string_lossy();
        let proc_flag = format!("--procedure={}", procedure);
        let raw = self.run_rea_command("decompile", &[&path_str, &proc_flag, "--provider", provider])?;
        Ok(raw)
    }
}
