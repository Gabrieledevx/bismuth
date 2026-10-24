use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Gestor de la estructura de carpetas de salida
#[derive(Debug, Clone)]
pub struct OutputLayout {
    pub base_dir: PathBuf,
    pub analysis_dir: PathBuf,
    pub pseudocode_dir: PathBuf,
    pub extracted_dir: PathBuf,
    pub resources_dir: PathBuf,
    pub metadata_dir: PathBuf,
    pub reconstructed_dir: PathBuf,
    pub reconstructed_src_dir: PathBuf,
    pub reconstructed_modules_dir: PathBuf,
    pub reconstructed_tests_dir: PathBuf,
    pub build_dir: PathBuf,
    pub preserved_dir: PathBuf,
    pub validation_dir: PathBuf,
}

impl OutputLayout {
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self> {
        let base = base_dir.as_ref().to_path_buf();

        let analysis_dir = base.join("analysis");
        let pseudocode_dir = analysis_dir.join("pseudocode");

        let extracted_dir = base.join("extracted");
        let resources_dir = extracted_dir.join("resources");
        let metadata_dir = extracted_dir.join("metadata");

        let reconstructed_dir = base.join("reconstructed");
        let reconstructed_src_dir = reconstructed_dir.join("src");
        let reconstructed_modules_dir = reconstructed_src_dir.join("modules");
        let reconstructed_tests_dir = reconstructed_dir.join("tests");

        let build_dir = base.join("build");
        let preserved_dir = base.join("preserved");
        let validation_dir = base.join("validation");

        let layout = Self {
            base_dir: base,
            analysis_dir,
            pseudocode_dir,
            extracted_dir,
            resources_dir,
            metadata_dir,
            reconstructed_dir,
            reconstructed_src_dir,
            reconstructed_modules_dir,
            reconstructed_tests_dir,
            build_dir,
            preserved_dir,
            validation_dir,
        };

        layout.ensure_directories()?;
        Ok(layout)
    }

    pub fn ensure_directories(&self) -> Result<()> {
        fs::create_dir_all(&self.analysis_dir).context("Error creando directorio analysis")?;
        fs::create_dir_all(&self.pseudocode_dir).context("Error creando directorio pseudocode")?;
        fs::create_dir_all(&self.resources_dir).context("Error creando directorio resources")?;
        fs::create_dir_all(&self.metadata_dir).context("Error creando directorio metadata")?;
        fs::create_dir_all(&self.reconstructed_modules_dir)
            .context("Error creando directorio reconstructed/src/modules")?;
        fs::create_dir_all(&self.reconstructed_tests_dir)
            .context("Error creando directorio reconstructed/tests")?;
        fs::create_dir_all(&self.build_dir).context("Error creando directorio build")?;
        fs::create_dir_all(&self.preserved_dir).context("Error creando directorio preserved")?;
        fs::create_dir_all(&self.validation_dir).context("Error creando directorio validation")?;
        Ok(())
    }

    pub fn preserve_original(&self, source: &Path) -> Result<PathBuf> {
        let target = self.preserved_dir.join("original.exe");
        fs::copy(source, &target).with_context(|| {
            format!(
                "Error preservando el ejecutable original en {}",
                target.display()
            )
        })?;
        Ok(target)
    }

    pub fn write_analysis_file(&self, filename: &str, content: &str) -> Result<PathBuf> {
        let path = self.analysis_dir.join(filename);
        fs::write(&path, content)
            .with_context(|| format!("Error escribiendo archivo {}", path.display()))?;
        Ok(path)
    }

    pub fn write_metadata_file(&self, filename: &str, content: &str) -> Result<PathBuf> {
        let path = self.metadata_dir.join(filename);
        fs::write(&path, content)
            .with_context(|| format!("Error escribiendo metadato {}", path.display()))?;
        Ok(path)
    }

    pub fn write_pseudocode(&self, func_name: &str, code: &str) -> Result<PathBuf> {
        let path = self.pseudocode_dir.join(format!("{}.c", func_name));
        fs::write(&path, code)
            .with_context(|| format!("Error escribiendo pseudocódigo {}", path.display()))?;
        Ok(path)
    }

    pub fn write_validation_report(&self, content: &str) -> Result<PathBuf> {
        let path = self.validation_dir.join("compatibility_report.md");
        fs::write(&path, content)
            .with_context(|| format!("Error escribiendo reporte {}", path.display()))?;
        Ok(path)
    }
}
