use crate::experiment::ExperimentRecord;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Project {
    pub root: PathBuf,
}

impl Project {
    pub fn discover(start: &Path) -> anyhow::Result<Self> {
        let mut dir = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
        loop {
            if dir.join("BOARD.md").exists() {
                return Ok(Self { root: dir });
            }
            if !dir.pop() {
                anyhow::bail!(
                    "no BOARD.md found in {} or any parent directory. Run `evidence-loop init` first.",
                    start.display()
                );
            }
        }
    }

    pub fn board_path(&self) -> PathBuf {
        self.root.join("BOARD.md")
    }

    pub fn control_dir(&self) -> PathBuf {
        self.root.join(".evidence-loop")
    }

    pub fn experiments_dir(&self) -> PathBuf {
        self.control_dir().join("experiments")
    }

    pub fn experiment_path(&self, id: &str) -> PathBuf {
        self.experiments_dir().join(format!("{id}.yml"))
    }

    pub fn current_pointer_path(&self) -> PathBuf {
        self.control_dir().join("current")
    }

    pub fn init(root: &Path) -> anyhow::Result<Self> {
        fs::create_dir_all(root.join(".evidence-loop").join("experiments"))?;
        let board_path = root.join("BOARD.md");
        if !board_path.exists() {
            fs::write(&board_path, crate::board::render_empty())?;
        }
        Ok(Self { root: root.to_path_buf() })
    }

    pub fn current_experiment_id(&self) -> Option<String> {
        fs::read_to_string(self.current_pointer_path())
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    pub fn set_current_experiment(&self, id: &str) -> anyhow::Result<()> {
        fs::create_dir_all(self.control_dir())?;
        fs::write(self.current_pointer_path(), id)?;
        Ok(())
    }

    pub fn next_experiment_id(&self) -> anyhow::Result<String> {
        let dir = self.experiments_dir();
        let mut max = 0u32;
        if dir.exists() {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if let Some(stem) = name.strip_suffix(".yml") {
                    if let Some(num) = stem.strip_prefix('E').and_then(|n| n.parse::<u32>().ok()) {
                        max = max.max(num);
                    }
                }
            }
        }
        Ok(format!("E{:03}", max + 1))
    }

    pub fn load_experiment(&self, id: &str) -> anyhow::Result<ExperimentRecord> {
        let path = self.experiment_path(id);
        if !path.exists() {
            anyhow::bail!("no such experiment: {id}");
        }
        ExperimentRecord::load(&path)
    }

    pub fn save_experiment(&self, record: &ExperimentRecord) -> anyhow::Result<()> {
        record.save(&self.experiment_path(&record.id))
    }
}
