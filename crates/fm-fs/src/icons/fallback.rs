use fm_core::{SystemIconBucket, SystemIconKey, SystemIconPayload};

pub(super) fn resolve_system_icon(
    _key: &SystemIconKey,
    _bucket: SystemIconBucket,
) -> Option<SystemIconPayload> {
    None
}

pub(super) fn resolve_icon_by_hint(
    _icon_hint: &str,
    _bucket: SystemIconBucket,
) -> Option<SystemIconPayload> {
    None
}
