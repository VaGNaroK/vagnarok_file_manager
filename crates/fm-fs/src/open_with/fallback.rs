use std::path::Path;

use compact_str::CompactString;
use fm_core::{FsError, OpenWithAssociation, OpenWithCatalog};
use tokio_util::sync::CancellationToken;

pub(super) fn discover_open_with_catalog(
    path: &Path,
    mime_hint: Option<&str>,
    _cancel: &CancellationToken,
) -> Result<OpenWithCatalog, FsError> {
    let key = super::open_with_type_key_for_path(path, mime_hint);
    Ok(OpenWithCatalog {
        type_key: CompactString::from(key.as_str()),
        type_label: CompactString::from(key),
        system_default: None,
        recommended: Vec::new(),
        others: Vec::new(),
    })
}

pub(super) fn save_system_default(
    _type_key: &str,
    _assoc: &OpenWithAssociation,
) -> Result<(), FsError> {
    Ok(())
}

pub(super) fn reset_system_default(_type_key: &str) -> Result<(), FsError> {
    Ok(())
}

pub(super) fn launch_open_with(path: &Path, _target: &OpenWithAssociation) -> Result<(), FsError> {
    open::that_detached(path).map_err(|e| crate::map_io_err(path, e))
}
