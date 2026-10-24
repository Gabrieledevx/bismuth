use crate::fs_layout::OutputLayout;
use crate::triage::TriageResult;
use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

pub struct ProjectBuilder<'a> {
    layout: &'a OutputLayout,
    triage: &'a TriageResult,
}

impl<'a> ProjectBuilder<'a> {
    pub fn new(layout: &'a OutputLayout, triage: &'a TriageResult) -> Self {
        Self { layout, triage }
    }

    /// Compila el proyecto Rust generado y coloca el nuevo binario en output/build/
    pub fn build_and_validate(&self) -> Result<bool> {
        println!("[+] Ejecutando cargo check en proyecto reconstruido...");
        let check_status = Command::new("cargo")
            .arg("check")
            .current_dir(&self.layout.reconstructed_dir)
            .status()
            .context("Fallo ejecutando cargo check")?;

        if !check_status.success() {
            self.write_report(false, "cargo check falló con errores de compilación.")?;
            return Ok(false);
        }

        println!("[+] Compilando binario con cargo build --release...");
        let build_output = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(&self.layout.reconstructed_dir)
            .output()
            .context("Fallo ejecutando cargo build --release")?;

        let success = build_output.status.success();
        let details = if success {
            // Copiar el ejecutable generado a output/build/reconstructed.exe
            let built_exe = self
                .layout
                .reconstructed_dir
                .join("target")
                .join("release")
                .join("reconstructed_app.exe");

            let target_exe = self.layout.build_dir.join("reconstructed.exe");
            if built_exe.exists() {
                fs::copy(&built_exe, &target_exe)?;
                format!(
                    "Binario compilado exitosamente y ubicado en: {}",
                    target_exe.display()
                )
            } else {
                "Compilación finalizada pero el archivo .exe no fue encontrado en target/release/"
                    .to_string()
            }
        } else {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            format!("Error de compilación:\n```\n{}\n```", stderr)
        };

        self.write_report(success, &details)?;
        Ok(success)
    }

    fn write_report(&self, success: bool, details: &str) -> Result<()> {
        let status_badge = if success {
            "✅ **COMPILACIÓN EXITOSA**"
        } else {
            "❌ **COMPILACIÓN CON ERRORES**"
        };

        let report = format!(
            r#"# Informe de Validación y Reconstrucción

- **Binario original analizado**: `{original}`
- **Arquitectura**: {arch}
- **Tecnología clasificada**: {tech:?}
- **Nivel de confianza del análisis**: {confidence:.1}%
- **Estado de compilación**: {status}

---

## 1. Evidencias e Importaciones de Windows
{apis}

## 2. Notas y Limitaciones de Ingeniería Inversa
{notes}

## 3. Resultado de la Construcción
{details}

## 4. Artefacto Preservado
{preserved}
"#,
            original = self.triage.target_file,
            arch = self.triage.architecture,
            tech = self.triage.technology,
            confidence = self.triage.confidence * 100.0,
            status = status_badge,
            apis = self
                .triage
                .imported_apis
                .iter()
                .map(|a| format!("- **{}**: {} APIs", a.dll, a.functions.len()))
                .collect::<Vec<_>>()
                .join("\n"),
            notes = self
                .triage
                .notes
                .iter()
                .map(|n| format!("- {}", n))
                .collect::<Vec<_>>()
                .join("\n"),
            details = details,
            preserved = if let Some(runtime) = &self.triage.runtime_dir {
                format!(
                    "- Runtime completo preservado en: `{}`\n- `reconstructed.exe` funciona como launcher del `BurpSuite.exe` preservado.",
                    runtime
                )
            } else if self.triage.technology == crate::triage::BinaryTechnology::Install4jJava {
                format!(
                    "- Ejecutable original preservado en: `{}`\n- `reconstructed.exe` funciona como launcher del original preservado.",
                    self.layout.preserved_dir.join("original.exe").display()
                )
            } else {
                "- No aplica para esta tecnología.".to_string()
            }
        );

        self.layout.write_validation_report(&report)?;
        Ok(())
    }
}
