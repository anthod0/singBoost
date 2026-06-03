use crate::core::paths::AppPaths;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};

pub struct RuntimeLog {
    file: File,
}

impl RuntimeLog {
    pub fn recreate(paths: &AppPaths) -> io::Result<Self> {
        std::fs::create_dir_all(paths.logs_dir())?;
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(paths.runtime_log())?;
        Ok(Self { file })
    }

    pub fn append_event(&mut self, message: impl AsRef<str>) -> io::Result<()> {
        writeln!(self.file, "[{}] {}", timestamp(), message.as_ref())?;
        self.file.flush()
    }
}

fn timestamp() -> String {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs().to_string(),
        Err(_) => "0".to_string(),
    }
}
