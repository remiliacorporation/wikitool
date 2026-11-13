use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::support::normalize_path;

/// The exact local bytes an upload plan binds. The file may live anywhere on
/// disk: uploaded media is not part of the project's page-sync mirror.
#[derive(Debug, Clone)]
pub struct LocalUploadSource {
    pub local_path: String,
    pub bytes: Vec<u8>,
    pub text: Option<String>,
}

/// Read the media file and optional description-page wikitext. This function
/// is read-only and is repeated at apply so drift changes the plan ID.
pub fn read_local_upload_source(
    file: &Path,
    text_file: Option<&Path>,
) -> Result<LocalUploadSource> {
    let absolute = std::path::absolute(file)
        .with_context(|| format!("failed to resolve {}", file.display()))?;
    if !absolute.is_file() {
        bail!("upload source {} is not a file", normalize_path(&absolute));
    }
    let bytes =
        fs::read(&absolute).with_context(|| format!("failed to read {}", absolute.display()))?;
    let text = text_file
        .map(|path| {
            fs::read_to_string(path).with_context(|| {
                format!(
                    "failed to read description wikitext {} as UTF-8",
                    path.display()
                )
            })
        })
        .transpose()?;
    Ok(LocalUploadSource {
        local_path: normalize_path(&absolute),
        bytes,
        text,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::read_local_upload_source;

    #[test]
    fn local_upload_source_binds_exact_bytes_and_an_absolute_path() {
        let temp = tempdir().expect("tempdir");
        let file = temp.path().join("photo.jpeg");
        let text = temp.path().join("photo.wiki");
        fs::write(&file, [0xFF, 0xD8, 0xFF, 0x00]).expect("media");
        fs::write(&text, "== Summary ==\n").expect("text");

        let source = read_local_upload_source(&file, Some(&text)).expect("source");
        assert_eq!(source.bytes, vec![0xFF, 0xD8, 0xFF, 0x00]);
        assert_eq!(source.text.as_deref(), Some("== Summary ==\n"));
        assert!(std::path::Path::new(&source.local_path).is_absolute());
        assert!(!source.local_path.contains('\\'));

        assert!(read_local_upload_source(temp.path(), None).is_err());
        assert!(read_local_upload_source(&file, Some(&temp.path().join("missing.wiki"))).is_err());
    }
}
