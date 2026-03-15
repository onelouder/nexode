use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crc32fast::hash as crc32;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeOutcomeTag {
    Success,
    Conflict,
    VerificationFailed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WalEntry {
    SessionStarted {
        timestamp_ms: u64,
        session_config_hash: [u8; 32],
        daemon_instance_id: String,
    },
    SlotStateChanged {
        timestamp_ms: u64,
        slot_id: String,
        project_id: String,
        task_status: i32,
        agent_id: Option<String>,
        agent_pid: Option<u32>,
        worktree_path: Option<String>,
    },
    TelemetryRecorded {
        timestamp_ms: u64,
        slot_id: String,
        project_id: String,
        tokens_in: u64,
        tokens_out: u64,
        cost_usd: f64,
    },
    MergeCompleted {
        timestamp_ms: u64,
        slot_id: String,
        project_id: String,
        outcome: MergeOutcomeTag,
    },
    Checkpoint {
        timestamp_ms: u64,
        full_state: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct WalReadResult {
    pub entries: Vec<WalEntry>,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct Wal {
    path: PathBuf,
    file: File,
}

#[derive(Debug, Error)]
pub enum WalError {
    #[error("io error at `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize WAL entry: {0}")]
    Encode(#[from] Box<bincode::ErrorKind>),
    #[error("failed to deserialize WAL entry: {0}")]
    Decode(Box<bincode::ErrorKind>),
    #[error("WAL entry length {0} exceeds u32 framing limit")]
    EntryTooLarge(usize),
}

impl Wal {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, WalError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| WalError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)
            .map_err(|source| WalError::Io {
                path: path.clone(),
                source,
            })?;

        Ok(Self { path, file })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_empty(&self) -> Result<bool, WalError> {
        let metadata = fs::metadata(&self.path).map_err(|source| WalError::Io {
            path: self.path.clone(),
            source,
        })?;
        Ok(metadata.len() == 0)
    }

    pub fn append(&mut self, entry: &WalEntry) -> Result<(), WalError> {
        let payload = bincode::serialize(entry)?;
        self.append_payload(&payload)
    }

    pub fn read_all(&self) -> Result<WalReadResult, WalError> {
        let bytes = fs::read(&self.path).map_err(|source| WalError::Io {
            path: self.path.clone(),
            source,
        })?;
        let mut cursor = 0usize;
        let mut entries = Vec::new();
        let mut warnings = Vec::new();

        while cursor < bytes.len() {
            if bytes.len() - cursor < 8 {
                warnings.push(format!(
                    "truncated WAL header at byte {} in `{}`; stopping replay",
                    cursor,
                    self.path.display()
                ));
                break;
            }

            let len = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().expect("u32 slice"))
                as usize;
            let expected_crc = u32::from_le_bytes(
                bytes[cursor + 4..cursor + 8]
                    .try_into()
                    .expect("u32 slice"),
            );
            cursor += 8;

            if bytes.len() - cursor < len {
                warnings.push(format!(
                    "truncated WAL payload of {} bytes at byte {} in `{}`; stopping replay",
                    len,
                    cursor,
                    self.path.display()
                ));
                break;
            }

            let payload = &bytes[cursor..cursor + len];
            cursor += len;

            let actual_crc = crc32(payload);
            if actual_crc != expected_crc {
                warnings.push(format!(
                    "CRC mismatch for WAL entry ending at byte {} in `{}`; skipping entry",
                    cursor,
                    self.path.display()
                ));
                continue;
            }

            match bincode::deserialize::<WalEntry>(payload) {
                Ok(entry) => entries.push(entry),
                Err(error) => warnings.push(format!(
                    "failed to decode WAL entry ending at byte {} in `{}`: {}; skipping entry",
                    cursor,
                    self.path.display(),
                    error
                )),
            }
        }

        Ok(WalReadResult { entries, warnings })
    }

    pub fn compact_to_checkpoint(
        &mut self,
        checkpoint: &WalEntry,
    ) -> Result<(), WalError> {
        let payload = bincode::serialize(checkpoint)?;
        let prev_path = self.path.with_extension("binlog.prev");

        if prev_path.exists() {
            fs::remove_file(&prev_path).map_err(|source| WalError::Io {
                path: prev_path.clone(),
                source,
            })?;
        }

        if self.path.exists() {
            fs::rename(&self.path, &prev_path).map_err(|source| WalError::Io {
                path: self.path.clone(),
                source,
            })?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&self.path)
            .map_err(|source| WalError::Io {
                path: self.path.clone(),
                source,
            })?;
        write_framed(&mut file, &self.path, &payload)?;
        self.file = file;

        if prev_path.exists() {
            fs::remove_file(&prev_path).map_err(|source| WalError::Io {
                path: prev_path,
                source,
            })?;
        }

        Ok(())
    }

    fn append_payload(&mut self, payload: &[u8]) -> Result<(), WalError> {
        write_framed(&mut self.file, &self.path, payload)
    }
}

fn write_framed(file: &mut File, path: &Path, payload: &[u8]) -> Result<(), WalError> {
    let len = u32::try_from(payload.len()).map_err(|_| WalError::EntryTooLarge(payload.len()))?;
    let crc = crc32(payload);

    file.write_all(&len.to_le_bytes())
        .and_then(|_| file.write_all(&crc.to_le_bytes()))
        .and_then(|_| file.write_all(payload))
        .and_then(|_| file.sync_all())
        .map_err(|source| WalError::Io {
            path: path.to_path_buf(),
            source,
        })
}

pub fn resolve_wal_path(session_path: &Path) -> PathBuf {
    session_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".nexode")
        .join("wal.binlog")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_reads_framed_entries_in_order() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("wal.binlog");
        let mut wal = Wal::open(&path).expect("open wal");

        wal.append(&WalEntry::SessionStarted {
            timestamp_ms: 1,
            session_config_hash: [7; 32],
            daemon_instance_id: "instance-1".to_string(),
        })
        .expect("append session start");
        wal.append(&WalEntry::SlotStateChanged {
            timestamp_ms: 2,
            slot_id: "slot-a".to_string(),
            project_id: "project-1".to_string(),
            task_status: 2,
            agent_id: Some("agent-a".to_string()),
            agent_pid: Some(41),
            worktree_path: Some("/tmp/worktree".to_string()),
        })
        .expect("append slot state");

        let read = wal.read_all().expect("read wal");
        assert!(read.warnings.is_empty());
        assert_eq!(read.entries.len(), 2);
        assert!(matches!(read.entries[0], WalEntry::SessionStarted { .. }));
        assert!(matches!(read.entries[1], WalEntry::SlotStateChanged { .. }));
    }

    #[test]
    fn skips_crc_mismatches_without_crashing() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("wal.binlog");
        let mut wal = Wal::open(&path).expect("open wal");
        wal.append(&WalEntry::TelemetryRecorded {
            timestamp_ms: 3,
            slot_id: "slot-a".to_string(),
            project_id: "project-1".to_string(),
            tokens_in: 5,
            tokens_out: 2,
            cost_usd: 0.1,
        })
        .expect("append telemetry");

        let mut bytes = fs::read(&path).expect("read raw wal");
        *bytes.last_mut().expect("wal has payload") ^= 0xff;
        fs::write(&path, bytes).expect("corrupt wal");

        let read = wal.read_all().expect("read wal");
        assert!(read.entries.is_empty());
        assert_eq!(read.warnings.len(), 1);
        assert!(read.warnings[0].contains("CRC mismatch"));
    }

    #[test]
    fn compaction_rewrites_the_wal_to_a_single_checkpoint() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("wal.binlog");
        let mut wal = Wal::open(&path).expect("open wal");
        wal.append(&WalEntry::SessionStarted {
            timestamp_ms: 1,
            session_config_hash: [1; 32],
            daemon_instance_id: "instance-1".to_string(),
        })
        .expect("append session start");

        let checkpoint = WalEntry::Checkpoint {
            timestamp_ms: 2,
            full_state: vec![1, 2, 3],
        };
        wal.compact_to_checkpoint(&checkpoint)
            .expect("compact to checkpoint");

        let read = wal.read_all().expect("read compacted wal");
        assert_eq!(read.entries, vec![checkpoint]);
    }
}
