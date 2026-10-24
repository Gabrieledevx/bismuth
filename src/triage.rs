use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BinaryTechnology {
    NativeWin32,
    DotNet,
    Install4jJava,
    PyInstaller,
    ElectronNode,
    Rust,
    Go,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedApi {
    pub dll: String,
    pub functions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageResult {
    pub target_file: String,
    pub runtime_dir: Option<String>,
    pub technology: BinaryTechnology,
    pub architecture: String,
    pub is_gui: bool,
    pub imported_apis: Vec<ImportedApi>,
    pub signer_organization: Option<String>,
    pub confidence: f32, // 0.0 a 1.0
    pub notes: Vec<String>,
}

pub struct TriageAnalyzer;

impl TriageAnalyzer {
    pub fn analyze(
        target_path: &str,
        imports: &serde_json::Value,
        resources: &serde_json::Value,
        certificates: &serde_json::Value,
        artifact: Option<&serde_json::Value>,
    ) -> TriageResult {
        let mut notes = Vec::new();
        let mut imported_apis = Vec::new();
        let mut all_dlls = HashSet::new();

        // 1. Extraer DLLs y funciones importadas
        if let Some(descriptors) = imports.get("descriptors").and_then(|d| d.as_array()) {
            for desc in descriptors {
                if let Some(dll_name) = desc.get("dll").and_then(|d| d.as_str()) {
                    let dll_lower = dll_name.to_lowercase();
                    all_dlls.insert(dll_lower.clone());

                    let mut funcs = Vec::new();
                    if let Some(entries) = desc.get("entries").and_then(|e| e.as_array()) {
                        for entry in entries {
                            if let Some(fn_name) = entry.get("name").and_then(|n| n.as_str()) {
                                funcs.push(fn_name.to_string());
                            }
                        }
                    }

                    imported_apis.push(ImportedApi {
                        dll: dll_name.to_string(),
                        functions: funcs,
                    });
                }
            }
        }

        // 2. Extraer certificado Authenticode si existe
        let mut signer_org = None;
        if let Some(signers) = certificates.get("signers").and_then(|s| s.as_array()) {
            if let Some(first_signer) = signers.first() {
                if let Some(subject) = first_signer.get("subject").and_then(|s| s.as_str()) {
                    signer_org = Some(subject.to_string());
                    notes.push(format!("Firma digital Authenticode detectada: {}", subject));
                }
            }
        }

        // 3. Detectar tecnología
        let mut technology = BinaryTechnology::NativeWin32;
        let mut confidence = 0.85;

        // Comprobación de .NET
        if all_dlls.contains("mscoree.dll") {
            technology = BinaryTechnology::DotNet;
            notes.push("Detectado runtime .NET CLR por importación de mscoree.dll".to_string());
        }
        // Comprobación de install4j / Java
        else if target_path.to_lowercase().contains("burp")
            || target_path.to_lowercase().contains("install4j")
            || serde_json::to_string(resources)
                .unwrap_or_default()
                .contains("install4j")
            || serde_json::to_string(resources)
                .unwrap_or_default()
                .contains("i4j")
        {
            technology = BinaryTechnology::Install4jJava;
            confidence = 0.95;
            notes.push("Detectado empaquetador install4j (Bootloader de aplicación Java). El ejecutable contiene JVM y archivos .jar embebidos.".to_string());
        }
        // Comprobación de PyInstaller
        else if serde_json::to_string(artifact.unwrap_or(&serde_json::Value::Null))
            .unwrap_or_default()
            .contains("pyinstaller")
        {
            technology = BinaryTechnology::PyInstaller;
            notes.push(
                "Detectado contenedor PyInstaller CArchive con código Python empaquetado."
                    .to_string(),
            );
        }
        // Comprobación de Electron
        else if all_dlls.contains("node.dll") || all_dlls.contains("ffmpeg.dll") {
            technology = BinaryTechnology::ElectronNode;
            notes.push("Detectado framework Electron / Node.js.".to_string());
        } else {
            notes.push("Binario nativo Win32/C++ identificado.".to_string());
        }

        // 4. Detectar subsistema GUI vs Consola
        let is_gui = all_dlls.contains("user32.dll") || all_dlls.contains("gdi32.dll");
        let architecture = imports
            .get("subject")
            .and_then(|subject| subject.get("architecture"))
            .and_then(|architecture| architecture.as_str())
            .or_else(|| {
                resources
                    .get("subject")
                    .and_then(|subject| subject.get("architecture"))
                    .and_then(|architecture| architecture.as_str())
            })
            .map(|architecture| match architecture {
                "x86" => "x86 (32-bit)".to_string(),
                "x86_64" => "x86_64 (64-bit)".to_string(),
                other => other.to_string(),
            })
            .unwrap_or_else(|| {
                if resources
                    .get("normalized_result")
                    .and_then(|result| result.get("format"))
                    .and_then(|format| format.as_str())
                    == Some("pe32")
                {
                    "x86 (32-bit)".to_string()
                } else {
                    "x86_64 (64-bit)".to_string()
                }
            });

        TriageResult {
            target_file: target_path.to_string(),
            runtime_dir: None,
            technology,
            architecture,
            is_gui,
            imported_apis,
            signer_organization: signer_org,
            confidence,
            notes,
        }
    }
}
