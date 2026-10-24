use anyhow::{Context, Result};
use goblin::pe::PE;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Extracts the PE import table locally, without requiring Ghidra, IDA or REA providers.
pub fn inspect_imports(path: &Path) -> Result<Value> {
    let bytes =
        fs::read(path).with_context(|| format!("No se pudo leer PE: {}", path.display()))?;
    let pe = PE::parse(&bytes).context("El archivo no contiene una estructura PE válida")?;
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for import in pe.imports {
        grouped
            .entry(import.dll.to_string())
            .or_default()
            .push(import.name.to_string());
    }

    let descriptors = grouped
        .into_iter()
        .map(|(dll, functions)| {
            json!({
                "dll": dll,
                "entries": functions.into_iter().map(|name| json!({ "name": name })).collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "format": if pe.is_64 { "pe32-plus" } else { "pe32" },
        "subject": { "architecture": if pe.is_64 { "x86_64" } else { "x86" } },
        "provider": { "id": "bismuth-local-pe", "name": "Bismuth local PE parser" },
        "descriptors": descriptors
    }))
}

/// Reads the Windows Authenticode status through the built-in PowerShell API.
pub fn inspect_authenticode(path: &Path) -> Result<Value> {
    #[cfg(not(windows))]
    {
        let _ = path;
        return Ok(json!({ "signers": [], "availability": "unsupported-platform" }));
    }

    #[cfg(windows)]
    {
        let script = r#"
$signature = Get-AuthenticodeSignature -LiteralPath $env:BISMUTH_TARGET
$certificate = $signature.SignerCertificate
[pscustomobject]@{
  status = [string]$signature.Status
  status_message = [string]$signature.StatusMessage
  subject = if ($certificate) { [string]$certificate.Subject } else { $null }
  issuer = if ($certificate) { [string]$certificate.Issuer } else { $null }
  thumbprint = if ($certificate) { [string]$certificate.Thumbprint } else { $null }
} | ConvertTo-Json -Compress
"#;
        let output = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ])
            .env("BISMUTH_TARGET", path)
            .output()
            .context("No se pudo ejecutar PowerShell para consultar Authenticode")?;

        if !output.status.success() {
            anyhow::bail!(
                "PowerShell Authenticode falló: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }

        let raw = String::from_utf8_lossy(&output.stdout);
        let signature: Value = serde_json::from_str(raw.trim())
            .context("PowerShell no devolvió JSON Authenticode válido")?;
        let status = signature
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("Unknown");
        let signers = signature
            .get("subject")
            .and_then(Value::as_str)
            .filter(|subject| !subject.is_empty())
            .map(|subject| {
                vec![json!({
                    "subject": subject,
                    "issuer": signature.get("issuer").cloned().unwrap_or(Value::Null),
                    "thumbprint": signature.get("thumbprint").cloned().unwrap_or(Value::Null)
                })]
            })
            .unwrap_or_default();

        Ok(json!({
            "status": status,
            "status_message": signature.get("status_message").cloned().unwrap_or(Value::Null),
            "signers": signers,
            "provider": { "id": "windows-authenticode", "name": "PowerShell Get-AuthenticodeSignature" }
        }))
    }
}
