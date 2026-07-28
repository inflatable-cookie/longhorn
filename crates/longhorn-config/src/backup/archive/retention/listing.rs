use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
};

use crate::{
    BackupApplication, BackupArchiveError, BackupArchiveLimits, BackupKind,
    backup::types::parse_utc_timestamp,
};

use super::{
    BackupOperationalCandidate, BackupOperationalListing, BackupRetentionDiagnosticKind,
    HARD_MAX_SCAN_ENTRIES, diagnostic,
};
use crate::backup::archive::{
    BackupOperationalRoot, inspect_backup_archive, publication::read_bounded_archive,
};

/// Inventories one operational root without mutating it.
pub fn list_operational_backups(
    root: &BackupOperationalRoot,
    application: &BackupApplication,
    archive_limits: BackupArchiveLimits,
    max_scan_entries: usize,
) -> BackupOperationalListing {
    let mut listing = BackupOperationalListing {
        root: root.path().to_path_buf(),
        candidates: Vec::new(),
        diagnostics: Vec::new(),
        complete: true,
    };
    if max_scan_entries == 0 || max_scan_entries > HARD_MAX_SCAN_ENTRIES {
        listing.complete = false;
        listing.diagnostics.push(diagnostic(
            BackupRetentionDiagnosticKind::ScanLimit,
            None,
            "invalid scan bound",
        ));
        return listing;
    }
    let entries = match fs::read_dir(root.path()) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return listing,
        Err(error) => {
            listing.complete = false;
            listing.diagnostics.push(diagnostic(
                BackupRetentionDiagnosticKind::RootRead,
                Some(root.path().to_path_buf()),
                error.to_string(),
            ));
            return listing;
        }
    };
    for (index, entry) in entries.enumerate() {
        if index >= max_scan_entries {
            listing.complete = false;
            listing.diagnostics.push(diagnostic(
                BackupRetentionDiagnosticKind::ScanLimit,
                Some(root.path().to_path_buf()),
                format!("root contains more than {max_scan_entries} entries"),
            ));
            break;
        }
        match entry {
            Ok(entry) => inspect_root_entry(&mut listing, entry, application, archive_limits),
            Err(error) => {
                listing.complete = false;
                listing.diagnostics.push(diagnostic(
                    BackupRetentionDiagnosticKind::RootRead,
                    Some(root.path().to_path_buf()),
                    error.to_string(),
                ));
            }
        }
    }
    exclude_duplicate_archive_ids(&mut listing);
    listing.candidates.sort_by(compare_newest);
    listing
}

fn inspect_root_entry(
    listing: &mut BackupOperationalListing,
    entry: fs::DirEntry,
    application: &BackupApplication,
    limits: BackupArchiveLimits,
) {
    let path = entry.path();
    let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
        listing.diagnostics.push(diagnostic(
            BackupRetentionDiagnosticKind::Unmanaged,
            Some(path),
            "non-UTF-8 root entry",
        ));
        return;
    };
    if name.ends_with(".longhorn-backup.age") {
        listing.diagnostics.push(diagnostic(
            BackupRetentionDiagnosticKind::Locked,
            Some(path),
            "encrypted archive requires the encryption layer",
        ));
        return;
    }
    if !name.ends_with(".longhorn-backup") {
        listing.diagnostics.push(diagnostic(
            BackupRetentionDiagnosticKind::Unmanaged,
            Some(path),
            "entry is outside plaintext archive naming",
        ));
        return;
    }
    match entry.file_type() {
        Ok(kind) if kind.is_file() => {}
        Ok(_) => {
            listing.diagnostics.push(diagnostic(
                BackupRetentionDiagnosticKind::NonRegular,
                Some(path),
                "candidate is not a regular file",
            ));
            return;
        }
        Err(error) => {
            listing.diagnostics.push(diagnostic(
                BackupRetentionDiagnosticKind::Unreadable,
                Some(path),
                error.to_string(),
            ));
            return;
        }
    }
    let bytes = match read_bounded_archive(&path, limits) {
        Ok(bytes) => bytes,
        Err(error) => {
            listing.diagnostics.push(diagnostic(
                BackupRetentionDiagnosticKind::Unreadable,
                Some(path),
                error.to_string(),
            ));
            return;
        }
    };
    let inspection = match inspect_backup_archive(&bytes, limits) {
        Ok(inspection) => inspection,
        Err(error) => {
            let kind = match error {
                BackupArchiveError::UnsupportedFormat { .. }
                | BackupArchiveError::UnsupportedFormatVersion { .. } => {
                    BackupRetentionDiagnosticKind::UnknownFormat
                }
                _ => BackupRetentionDiagnosticKind::Corrupt,
            };
            listing
                .diagnostics
                .push(diagnostic(kind, Some(path), error.to_string()));
            return;
        }
    };
    let manifest = inspection.manifest();
    if manifest.application().id() != application.id() {
        listing.diagnostics.push(diagnostic(
            BackupRetentionDiagnosticKind::ForeignApplication,
            Some(path),
            format!(
                "archive application {} does not match {}",
                manifest.application().id(),
                application.id()
            ),
        ));
        return;
    }
    if manifest.kind() == BackupKind::UserExport {
        listing.diagnostics.push(diagnostic(
            BackupRetentionDiagnosticKind::UserExport,
            Some(path),
            "user export is never an operational retention candidate",
        ));
        return;
    }
    let created_timestamp =
        parse_utc_timestamp(manifest.created_at()).expect("inspection validated UTC timestamp");
    listing.candidates.push(BackupOperationalCandidate {
        path,
        archive_id: manifest.archive_id().into(),
        created_at: manifest.created_at().into(),
        created_timestamp,
        kind: manifest.kind(),
        archive_sha256: inspection.archive_sha256().clone(),
    });
}

fn exclude_duplicate_archive_ids(listing: &mut BackupOperationalListing) {
    let mut counts = BTreeMap::<String, usize>::new();
    for candidate in &listing.candidates {
        *counts.entry(candidate.archive_id.clone()).or_default() += 1;
    }
    let duplicate_ids = counts
        .into_iter()
        .filter_map(|(id, count)| (count > 1).then_some(id))
        .collect::<BTreeSet<_>>();
    if duplicate_ids.is_empty() {
        return;
    }
    let mut candidates = Vec::new();
    for candidate in listing.candidates.drain(..) {
        if duplicate_ids.contains(&candidate.archive_id) {
            listing.diagnostics.push(diagnostic(
                BackupRetentionDiagnosticKind::DuplicateArchiveId,
                Some(candidate.path),
                format!("duplicate archive id {}", candidate.archive_id),
            ));
        } else {
            candidates.push(candidate);
        }
    }
    listing.candidates = candidates;
}

fn compare_newest(
    left: &BackupOperationalCandidate,
    right: &BackupOperationalCandidate,
) -> std::cmp::Ordering {
    right
        .created_timestamp
        .cmp(&left.created_timestamp)
        .then_with(|| right.archive_id.cmp(&left.archive_id))
        .then_with(|| left.path.cmp(&right.path))
}
