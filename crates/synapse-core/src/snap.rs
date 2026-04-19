//! .brainpack export/import: zstd(sqlite file).

use crate::error::Result;
use rusqlite::Connection;
use std::path::Path;

const MAGIC: &[u8; 4] = b"BPK1";
const VERSION: u8 = 1;

/// Write a .brainpack file from a live Store path.
/// Does a proper SQLite backup to avoid WAL drift.
pub fn export(db_path: impl AsRef<Path>, out: impl AsRef<Path>, level: i32) -> Result<()> {
    let tmp = tempfile::NamedTempFile::new()?;
    let src = Connection::open(db_path.as_ref())?;
    let mut dst = Connection::open(tmp.path())?;
    let backup = rusqlite::backup::Backup::new(&src, &mut dst)?;
    backup.run_to_completion(128, std::time::Duration::from_millis(0), None)?;
    drop(backup);
    drop(dst);
    let data = std::fs::read(tmp.path())?;
    let blake = blake3::hash(&data);
    let compressed = zstd::encode_all(&data[..], level)?;
    let mut file = std::fs::File::create(out)?;
    use std::io::Write;
    file.write_all(MAGIC)?;
    file.write_all(&[VERSION])?;
    file.write_all(&(level as u32).to_le_bytes())?;
    file.write_all(&(data.len() as u64).to_le_bytes())?;
    file.write_all(blake.as_bytes())?;
    file.write_all(&compressed)?;
    Ok(())
}

/// Restore a .brainpack into a fresh db file at `out`.
pub fn import(pack: impl AsRef<Path>, out: impl AsRef<Path>) -> Result<()> {
    let raw = std::fs::read(pack)?;
    if &raw[0..4] != MAGIC {
        return Err(crate::Error::Other("bad magic".into()));
    }
    let _version = raw[4];
    let _level = u32::from_le_bytes(raw[5..9].try_into().unwrap());
    let raw_len = u64::from_le_bytes(raw[9..17].try_into().unwrap()) as usize;
    let hash = &raw[17..49];
    let body = &raw[49..];
    let data = zstd::decode_all(body)?;
    if data.len() != raw_len {
        return Err(crate::Error::Other("size mismatch".into()));
    }
    let actual = blake3::hash(&data);
    if actual.as_bytes() != hash {
        return Err(crate::Error::Other("blake3 mismatch".into()));
    }
    std::fs::write(out, data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PutRequest, Store};

    #[test]
    fn roundtrip() {
        let db = tempfile::NamedTempFile::new().unwrap();
        {
            let mut s = Store::open(db.path()).unwrap();
            s.put(&PutRequest { text: "round trip test".into(), ..Default::default() }).unwrap();
        }
        let pack = tempfile::NamedTempFile::new().unwrap();
        export(db.path(), pack.path(), 3).unwrap();
        let restored = tempfile::NamedTempFile::new().unwrap();
        import(pack.path(), restored.path()).unwrap();
        let s = Store::open(restored.path()).unwrap();
        assert_eq!(s.stats().unwrap().docs, 1);
    }
}
