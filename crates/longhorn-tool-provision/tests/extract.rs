//! Contract 027: bounded extraction of one executable, through the public
//! API only. Archives are built in fresh temporary directories.

mod support;

use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use longhorn_tool_provision::{
    ArtifactShape, Declaration, DigestOrigin, DigestSource, ExtractionLimits, Fault,
    ProvisionError, Refusal, SignatureReport, SignatureVerifier, StagedExecutable,
    VerifiedArtifact, extract_executable, verify_artifact, verify_signature,
};
use support::{TarEntry, ZipEntry, sha256_hex, tar_gz, write_file, zip_of};

const BINARY: &[u8] = b"#!/bin/sh\necho tool\n";

/// The archive, written into its own directory, verified by digest.
fn verified(dir: &Path, asset: &str, archive: &[u8]) -> VerifiedArtifact {
    let path = write_file(dir, asset, archive);
    let checksums = format!("{}  {asset}\n", sha256_hex(archive));
    verify_artifact(
        &path,
        asset,
        Declaration {
            digest: Some(DigestSource::ReleaseChecksums(&checksums)),
            vendor_team: None,
        },
    )
    .expect("the fixture digest matches")
}

/// A fresh staging directory inside `root`.
fn staging(root: &Path, name: &str) -> std::path::PathBuf {
    let dir = root.join(name);
    std::fs::create_dir(&dir).expect("staging directory");
    dir
}

fn limits() -> ExtractionLimits {
    ExtractionLimits::DEFAULT
}

fn refused(result: Result<StagedExecutable, ProvisionError>) -> Refusal {
    match result {
        Err(ProvisionError::Refused(refusal)) => refusal,
        Err(ProvisionError::Fault(fault)) => panic!("expected a refusal, got fault {fault}"),
        Ok(_) => panic!("expected a refusal, got an extracted executable"),
    }
}

fn tar_archive_refused(members: &[TarEntry<'_>]) -> Refusal {
    let temp = tempfile::tempdir().unwrap();
    let artifact = verified(temp.path(), "tool.tar.gz", &tar_gz(members));
    refused(extract_executable(
        &artifact,
        ArtifactShape::TarGz { member: "tool" },
        &staging(temp.path(), "out"),
        "tool",
        limits(),
    ))
}

fn file(name: &str) -> TarEntry<'_> {
    TarEntry::File {
        name,
        bytes: BINARY.to_vec(),
        mode: 0o755,
    }
}

#[test]
#[cfg(unix)]
fn a_tar_gz_member_is_written_with_the_executable_mode_not_the_archives() {
    let temp = tempfile::tempdir().unwrap();
    // A setuid bit in the archive must not reach the staged file.
    let archive = tar_gz(&[
        TarEntry::Dir { name: "./" },
        TarEntry::File {
            name: "./tool-2.0.0/bin/tool",
            bytes: BINARY.to_vec(),
            mode: 0o4755,
        },
    ]);
    let artifact = verified(temp.path(), "tool.tar.gz", &archive);
    let out = staging(temp.path(), "out");
    extract_executable(
        &artifact,
        ArtifactShape::TarGz {
            member: "tool-2.0.0/bin/tool",
        },
        &out,
        "tool",
        limits(),
    )
    .expect("the member extracts");
    assert_eq!(std::fs::read(out.join("tool")).unwrap(), BINARY);
    let mode = std::fs::metadata(out.join("tool"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o7777, 0o755, "the mode is set here, setuid dropped");
}

#[test]
fn a_zip_member_in_a_nested_directory_is_extracted() {
    let temp = tempfile::tempdir().unwrap();
    let archive = zip_of(&[
        ZipEntry::Dir {
            name: "tool_2.0.0_macOS_arm64/",
        },
        ZipEntry::File {
            name: "tool_2.0.0_macOS_arm64/bin/tool",
            bytes: BINARY.to_vec(),
        },
        ZipEntry::File {
            name: "tool_2.0.0_macOS_arm64/README",
            bytes: b"docs".to_vec(),
        },
    ]);
    let artifact = verified(temp.path(), "tool.zip", &archive);
    let out = staging(temp.path(), "out");
    extract_executable(
        &artifact,
        ArtifactShape::Zip {
            member: "tool_2.0.0_macOS_arm64/bin/tool",
        },
        &out,
        "tool",
        limits(),
    )
    .expect("the member extracts");
    assert_eq!(std::fs::read(out.join("tool")).unwrap(), BINARY);
    assert!(
        !out.join("README").exists(),
        "only the requested member is written"
    );
}

#[test]
#[cfg(unix)]
fn a_raw_artifact_is_staged_as_the_executable() {
    let temp = tempfile::tempdir().unwrap();
    let artifact = verified(temp.path(), "tool-darwin-arm64", BINARY);
    let out = staging(temp.path(), "out");
    extract_executable(&artifact, ArtifactShape::Raw, &out, "tool", limits())
        .expect("the raw artifact stages");
    assert_eq!(std::fs::read(out.join("tool")).unwrap(), BINARY);
    let mode = std::fs::metadata(out.join("tool"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o755);
}

#[test]
fn a_missing_member_refuses() {
    let temp = tempfile::tempdir().unwrap();
    let artifact = verified(temp.path(), "tool.tar.gz", &tar_gz(&[file("other")]));
    assert!(matches!(
        refused(extract_executable(
            &artifact,
            ArtifactShape::TarGz { member: "tool" },
            &staging(temp.path(), "out"),
            "tool",
            limits(),
        )),
        Refusal::MemberMissing { member, .. } if member == "tool"
    ));
}

#[test]
fn a_member_held_twice_refuses() {
    assert!(matches!(
        tar_archive_refused(&[file("tool"), file("tool")]),
        Refusal::DuplicateMember { .. }
    ));
}

#[test]
fn escaping_and_absolute_names_refuse_anywhere_in_the_archive() {
    for name in ["../escape", "/absolute/escape", "bin/../../escape"] {
        assert!(
            matches!(
                tar_archive_refused(&[file("tool"), file(name)]),
                Refusal::UnsafeEntry {
                    reason: Some(_),
                    ..
                }
            ),
            "{name}"
        );
    }
}

#[test]
fn a_backslash_in_a_name_refuses() {
    assert!(matches!(
        tar_archive_refused(&[file("tool"), file("bin\\escape")]),
        Refusal::UnsafeEntry { reason: None, .. }
    ));
    let temp = tempfile::tempdir().unwrap();
    let zip = zip_of(&[ZipEntry::File {
        name: "bin\\tool",
        bytes: BINARY.to_vec(),
    }]);
    let artifact = verified(temp.path(), "tool.zip", &zip);
    assert!(matches!(
        refused(extract_executable(
            &artifact,
            ArtifactShape::Zip { member: "bin/tool" },
            &staging(temp.path(), "out"),
            "tool",
            limits(),
        )),
        Refusal::UnsafeEntry { reason: None, .. }
    ));
}

#[test]
fn a_zip_entry_with_a_parent_component_refuses() {
    let temp = tempfile::tempdir().unwrap();
    let zip = zip_of(&[
        ZipEntry::File {
            name: "tool",
            bytes: BINARY.to_vec(),
        },
        ZipEntry::File {
            name: "../escape",
            bytes: b"x".to_vec(),
        },
    ]);
    let artifact = verified(temp.path(), "tool.zip", &zip);
    assert!(matches!(
        refused(extract_executable(
            &artifact,
            ArtifactShape::Zip { member: "tool" },
            &staging(temp.path(), "out"),
            "tool",
            limits(),
        )),
        Refusal::UnsafeEntry {
            reason: Some(_),
            ..
        }
    ));
}

#[test]
fn symlink_and_hard_link_members_refuse_even_when_they_are_not_the_member() {
    assert!(matches!(
        tar_archive_refused(&[
            file("tool"),
            TarEntry::Symlink {
                name: "link",
                target: "tool",
            },
        ]),
        Refusal::LinkEntry { .. }
    ));
    assert!(matches!(
        tar_archive_refused(&[
            file("tool"),
            TarEntry::HardLink {
                name: "hard",
                target: "tool",
            },
        ]),
        Refusal::LinkEntry { .. }
    ));
    let temp = tempfile::tempdir().unwrap();
    let zip = zip_of(&[
        ZipEntry::File {
            name: "tool",
            bytes: BINARY.to_vec(),
        },
        ZipEntry::Symlink {
            name: "link",
            target: "tool",
        },
    ]);
    let artifact = verified(temp.path(), "tool.zip", &zip);
    assert!(matches!(
        refused(extract_executable(
            &artifact,
            ArtifactShape::Zip { member: "tool" },
            &staging(temp.path(), "out"),
            "tool",
            limits(),
        )),
        Refusal::LinkEntry { .. }
    ));
}

#[test]
fn a_fifo_entry_refuses_as_unsupported() {
    assert!(matches!(
        tar_archive_refused(&[file("tool"), TarEntry::Fifo { name: "pipe" }]),
        Refusal::UnsupportedEntry { .. }
    ));
}

#[test]
fn the_member_count_bound_refuses_and_its_boundary_passes() {
    let temp = tempfile::tempdir().unwrap();
    let archive = tar_gz(&[file("a"), file("b"), file("tool")]);
    let artifact = verified(temp.path(), "tool.tar.gz", &archive);
    assert!(matches!(
        refused(extract_executable(
            &artifact,
            ArtifactShape::TarGz { member: "tool" },
            &staging(temp.path(), "over"),
            "tool",
            ExtractionLimits::new(2, u64::MAX),
        )),
        Refusal::TooManyMembers { limit: 2, .. }
    ));
    extract_executable(
        &artifact,
        ArtifactShape::TarGz { member: "tool" },
        &staging(temp.path(), "at"),
        "tool",
        ExtractionLimits::new(3, u64::MAX),
    )
    .expect("exactly the limit passes");

    let zip = zip_of(&[
        ZipEntry::File {
            name: "a",
            bytes: vec![],
        },
        ZipEntry::File {
            name: "b",
            bytes: vec![],
        },
        ZipEntry::File {
            name: "tool",
            bytes: BINARY.to_vec(),
        },
    ]);
    let artifact = verified(temp.path(), "tool.zip", &zip);
    assert!(matches!(
        refused(extract_executable(
            &artifact,
            ArtifactShape::Zip { member: "tool" },
            &staging(temp.path(), "zip-over"),
            "tool",
            ExtractionLimits::new(2, u64::MAX),
        )),
        Refusal::TooManyMembers { .. }
    ));
}

#[test]
fn skipped_members_count_against_the_decompressed_byte_bound() {
    // The target's declared size is 10 bytes, well under the bound. The 2 KiB
    // member before it is not the target, but the tar reader still
    // decompresses it, so the stream exceeds the bound.
    let temp = tempfile::tempdir().unwrap();
    let archive = tar_gz(&[
        TarEntry::File {
            name: "filler",
            bytes: vec![0u8; 2048],
            mode: 0o644,
        },
        TarEntry::File {
            name: "tool",
            bytes: vec![1u8; 10],
            mode: 0o755,
        },
    ]);
    let artifact = verified(temp.path(), "tool.tar.gz", &archive);
    assert!(matches!(
        refused(extract_executable(
            &artifact,
            ArtifactShape::TarGz { member: "tool" },
            &staging(temp.path(), "out"),
            "tool",
            ExtractionLimits::new(100, 1024),
        )),
        Refusal::ArchiveTooLarge { limit: 1024, .. }
    ));
}

#[test]
fn a_raw_artifact_over_the_byte_bound_refuses() {
    let temp = tempfile::tempdir().unwrap();
    let artifact = verified(temp.path(), "tool-darwin-arm64", &vec![0u8; 4096]);
    assert!(matches!(
        refused(extract_executable(
            &artifact,
            ArtifactShape::Raw,
            &staging(temp.path(), "out"),
            "tool",
            ExtractionLimits::new(100, 1024),
        )),
        Refusal::ArchiveTooLarge { .. }
    ));
}

/// Rewrites the declared uncompressed size of every zip record for `name` (the
/// local header and the central directory) to `declared`, so the header
/// claims less than the member really decompresses to.
fn declare_size(archive: &mut [u8], name: &str, declared: u32) {
    patch_records(
        archive,
        name,
        ZipField::UncompressedSize,
        &declared.to_le_bytes(),
    );
}

/// A field of a zip record, at its offset in the local and central headers.
#[derive(Clone, Copy)]
enum ZipField {
    Flags,
    Method,
    UncompressedSize,
}

impl ZipField {
    /// (local offset, central offset), per the zip format.
    const fn offsets(self) -> (usize, usize) {
        match self {
            Self::Flags => (6, 8),
            Self::Method => (8, 10),
            Self::UncompressedSize => (22, 24),
        }
    }
}

/// Overwrites `field` in both records (local and central) for `name`.
fn patch_records(archive: &mut [u8], name: &str, field: ZipField, value: &[u8]) {
    let (local, central) = field.offsets();
    // (signature, name-length offset, name offset, field offset) within the
    // record, per the zip format's local and central headers.
    let records = [
        (&b"PK\x03\x04"[..], 26usize, 30usize, local),
        (&b"PK\x01\x02"[..], 28, 46, central),
    ];
    let mut offsets = Vec::new();
    for (signature, length_offset, name_offset, field_offset) in records {
        for start in 0..archive.len().saturating_sub(name_offset) {
            if &archive[start..start + 4] != signature {
                continue;
            }
            let length_at = start + length_offset;
            let length = u16::from_le_bytes([archive[length_at], archive[length_at + 1]]) as usize;
            let stored = start + name_offset;
            if archive.get(stored..stored + length) == Some(name.as_bytes()) {
                offsets.push(start + field_offset);
            }
        }
    }
    assert_eq!(offsets.len(), 2, "the fixture holds the member twice");
    for offset in offsets {
        archive[offset..offset + value.len()].copy_from_slice(value);
    }
}

fn zip_refused(archive: &[u8]) -> Refusal {
    let temp = tempfile::tempdir().unwrap();
    let artifact = verified(temp.path(), "tool.zip", archive);
    let out = staging(temp.path(), "out");
    let refusal = refused(extract_executable(
        &artifact,
        ArtifactShape::Zip { member: "tool" },
        &out,
        "tool",
        limits(),
    ));
    assert_eq!(
        std::fs::read_dir(&out).unwrap().count(),
        0,
        "a refused zip leaves nothing behind"
    );
    refusal
}

const S_IFREG: u32 = 0o100000;

#[test]
fn a_zip_regular_file_with_a_trailing_slash_is_a_file_not_a_directory() {
    // The Unix mode says regular file; the name's slash does not override it.
    let temp = tempfile::tempdir().unwrap();
    let archive = zip_of(&[ZipEntry::Mode {
        name: "tool/",
        bytes: BINARY.to_vec(),
        mode: S_IFREG | 0o644,
    }]);
    let artifact = verified(temp.path(), "tool.zip", &archive);
    let out = staging(temp.path(), "out");
    extract_executable(
        &artifact,
        ArtifactShape::Zip { member: "tool" },
        &out,
        "tool",
        limits(),
    )
    .expect("the mode makes it the member");
    assert_eq!(std::fs::read(out.join("tool")).unwrap(), BINARY);

    let archive = zip_of(&[
        ZipEntry::File {
            name: "tool",
            bytes: BINARY.to_vec(),
        },
        ZipEntry::Mode {
            name: "tool/",
            bytes: b"other".to_vec(),
            mode: S_IFREG | 0o644,
        },
    ]);
    assert!(matches!(
        zip_refused(&archive),
        Refusal::DuplicateMember { .. }
    ));
}

#[test]
fn an_encrypted_member_anywhere_in_a_zip_refuses() {
    let mut archive = zip_of(&[
        ZipEntry::File {
            name: "tool",
            bytes: BINARY.to_vec(),
        },
        ZipEntry::File {
            name: "notes",
            bytes: b"secret".to_vec(),
        },
    ]);
    // General-purpose flag bit 0: the member is encrypted.
    patch_records(&mut archive, "notes", ZipField::Flags, &1u16.to_le_bytes());
    assert!(matches!(
        zip_refused(&archive),
        Refusal::UnreadableArchive { .. }
    ));
}

#[test]
fn an_unsupported_compression_method_anywhere_in_a_zip_refuses() {
    let mut archive = zip_of(&[
        ZipEntry::File {
            name: "tool",
            bytes: BINARY.to_vec(),
        },
        ZipEntry::File {
            name: "notes",
            bytes: b"notes".to_vec(),
        },
    ]);
    // Method 12 is bzip2, which this crate does not decode.
    patch_records(
        &mut archive,
        "notes",
        ZipField::Method,
        &12u16.to_le_bytes(),
    );
    assert!(matches!(
        zip_refused(&archive),
        Refusal::UnreadableArchive { .. }
    ));
}

#[test]
fn a_zip_member_whose_real_size_exceeds_the_bound_refuses_while_its_declared_size_does_not() {
    // The header claims 16 bytes; the member decompresses to 4096. The bound
    // counts what is produced, so the header cannot keep the bytes out.
    let temp = tempfile::tempdir().unwrap();
    let mut archive = zip_of(&[ZipEntry::File {
        name: "tool",
        bytes: vec![0u8; 4096],
    }]);
    declare_size(&mut archive, "tool", 16);
    let artifact = verified(temp.path(), "tool.zip", &archive);
    assert!(matches!(
        refused(extract_executable(
            &artifact,
            ArtifactShape::Zip { member: "tool" },
            &staging(temp.path(), "out"),
            "tool",
            ExtractionLimits::new(100, 1024),
        )),
        Refusal::ArchiveTooLarge { limit: 1024, .. }
    ));
}

#[test]
fn a_zip_member_with_an_absolute_name_refuses() {
    let temp = tempfile::tempdir().unwrap();
    let archive = zip_of(&[
        ZipEntry::File {
            name: "tool",
            bytes: BINARY.to_vec(),
        },
        ZipEntry::File {
            name: "/absolute/escape",
            bytes: b"x".to_vec(),
        },
    ]);
    let artifact = verified(temp.path(), "tool.zip", &archive);
    assert!(matches!(
        refused(extract_executable(
            &artifact,
            ArtifactShape::Zip { member: "tool" },
            &staging(temp.path(), "out"),
            "tool",
            limits(),
        )),
        Refusal::UnsafeEntry {
            reason: Some(_),
            ..
        }
    ));
}

#[test]
fn zip_special_file_types_refuse_while_an_explicit_regular_file_passes() {
    // The Unix mode decides the entry type, whatever the name says: a fifo, a
    // character device, a block device and a socket are not files.
    const S_IFCHR: u32 = 0o020000;
    const S_IFBLK: u32 = 0o060000;
    const S_IFIFO: u32 = 0o010000;
    const S_IFSOCK: u32 = 0o140000;
    for special in [S_IFIFO, S_IFCHR, S_IFBLK, S_IFSOCK] {
        let temp = tempfile::tempdir().unwrap();
        let archive = zip_of(&[ZipEntry::Mode {
            name: "tool",
            bytes: Vec::new(),
            mode: special | 0o644,
        }]);
        let artifact = verified(temp.path(), "tool.zip", &archive);
        assert!(
            matches!(
                refused(extract_executable(
                    &artifact,
                    ArtifactShape::Zip { member: "tool" },
                    &staging(temp.path(), "out"),
                    "tool",
                    limits(),
                )),
                Refusal::UnsupportedEntry { .. }
            ),
            "mode {special:o}"
        );
    }
    let temp = tempfile::tempdir().unwrap();
    let archive = zip_of(&[ZipEntry::Mode {
        name: "tool",
        bytes: BINARY.to_vec(),
        mode: S_IFREG | 0o644,
    }]);
    let artifact = verified(temp.path(), "tool.zip", &archive);
    extract_executable(
        &artifact,
        ArtifactShape::Zip { member: "tool" },
        &staging(temp.path(), "out"),
        "tool",
        limits(),
    )
    .expect("an explicitly regular file extracts");
}

#[test]
fn a_source_changed_after_verification_refuses_and_leaves_nothing_behind() {
    let temp = tempfile::tempdir().unwrap();
    let archive = tar_gz(&[file("tool")]);
    let artifact = verified(temp.path(), "tool.tar.gz", &archive);
    // The same path, different bytes, after the digest was checked.
    std::fs::write(
        temp.path().join("tool.tar.gz"),
        b"replaced after verification",
    )
    .unwrap();
    let out = staging(temp.path(), "out");
    assert!(matches!(
        refused(extract_executable(
            &artifact,
            ArtifactShape::TarGz { member: "tool" },
            &out,
            "tool",
            limits(),
        )),
        Refusal::DigestMismatch {
            origin: DigestOrigin::ReleaseChecksums,
            ..
        }
    ));
    assert_eq!(
        std::fs::read_dir(&out).unwrap().count(),
        0,
        "neither the staged file nor the private copy remains"
    );
}

#[test]
fn an_honest_zip_member_over_the_byte_bound_refuses_as_too_large() {
    // Declared and real sizes agree, so the budget is what refuses.
    let temp = tempfile::tempdir().unwrap();
    let archive = zip_of(&[ZipEntry::File {
        name: "tool",
        bytes: vec![0u8; 4096],
    }]);
    let artifact = verified(temp.path(), "tool.zip", &archive);
    assert!(matches!(
        refused(extract_executable(
            &artifact,
            ArtifactShape::Zip { member: "tool" },
            &staging(temp.path(), "out"),
            "tool",
            ExtractionLimits::new(100, 1024),
        )),
        Refusal::ArchiveTooLarge { limit: 1024, .. }
    ));
}

#[test]
fn an_unreadable_archive_refuses_as_unreadable() {
    let temp = tempfile::tempdir().unwrap();
    let garbage = b"not an archive at all".to_vec();
    let tar = verified(temp.path(), "tool.tar.gz", &garbage);
    assert!(matches!(
        refused(extract_executable(
            &tar,
            ArtifactShape::TarGz { member: "tool" },
            &staging(temp.path(), "tar-out"),
            "tool",
            limits(),
        )),
        Refusal::UnreadableArchive { .. }
    ));
    let zip = verified(temp.path(), "tool.zip", &garbage);
    assert!(matches!(
        refused(extract_executable(
            &zip,
            ArtifactShape::Zip { member: "tool" },
            &staging(temp.path(), "zip-out"),
            "tool",
            limits(),
        )),
        Refusal::UnreadableArchive { .. }
    ));
}

#[test]
fn a_staged_name_that_is_not_one_file_name_is_a_usage_fault() {
    let temp = tempfile::tempdir().unwrap();
    let artifact = verified(temp.path(), "tool-darwin-arm64", BINARY);
    let error = extract_executable(
        &artifact,
        ArtifactShape::Raw,
        &staging(temp.path(), "out"),
        "nested/tool",
        limits(),
    )
    .unwrap_err();
    assert!(
        matches!(error, ProvisionError::Fault(Fault::Usage { .. })),
        "{error}"
    );
}

#[test]
fn a_member_path_that_escapes_is_a_usage_fault_not_an_archive_refusal() {
    let temp = tempfile::tempdir().unwrap();
    let artifact = verified(temp.path(), "tool.tar.gz", &tar_gz(&[file("tool")]));
    let error = extract_executable(
        &artifact,
        ArtifactShape::TarGz { member: "../tool" },
        &staging(temp.path(), "out"),
        "tool",
        limits(),
    )
    .unwrap_err();
    assert!(
        matches!(error, ProvisionError::Fault(Fault::Usage { .. })),
        "{error}"
    );
}

#[test]
fn an_existing_staged_file_is_a_fault_and_is_never_overwritten() {
    let temp = tempfile::tempdir().unwrap();
    let artifact = verified(temp.path(), "tool-darwin-arm64", BINARY);
    let out = staging(temp.path(), "out");
    write_file(&out, "tool", b"earlier");
    let error =
        extract_executable(&artifact, ArtifactShape::Raw, &out, "tool", limits()).unwrap_err();
    assert!(
        matches!(error, ProvisionError::Fault(Fault::Io { .. })),
        "{error}"
    );
    assert_eq!(std::fs::read(out.join("tool")).unwrap(), b"earlier");
}

#[test]
fn a_root_directory_entry_and_a_dot_slash_member_extract() {
    let temp = tempfile::tempdir().unwrap();
    let archive = tar_gz(&[
        TarEntry::Dir { name: "./" },
        TarEntry::File {
            name: "./tool",
            bytes: BINARY.to_vec(),
            mode: 0o755,
        },
    ]);
    let artifact = verified(temp.path(), "tool.tar.gz", &archive);
    extract_executable(
        &artifact,
        ArtifactShape::TarGz { member: "tool" },
        &staging(temp.path(), "out"),
        "tool",
        limits(),
    )
    .expect("a ./ root and a ./ member are ordinary tar output");
}

/// The contract 027 chain: digest, bounded extraction, team, installable.
#[test]
fn the_chain_verifies_extracts_and_checks_the_team_before_installable() {
    struct Vendor;
    impl SignatureVerifier for Vendor {
        fn report(&self, path: &Path) -> Result<SignatureReport, ProvisionError> {
            assert!(
                path.is_file(),
                "the signature is checked on the staged file"
            );
            Ok(SignatureReport::Team("TEAMID0001".to_owned()))
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let archive = tar_gz(&[file("tool-2.0.0/tool")]);
    let path = write_file(temp.path(), "tool.tar.gz", &archive);
    let checksums = format!("{}  tool.tar.gz\n", sha256_hex(&archive));
    let artifact = verify_artifact(
        &path,
        "tool.tar.gz",
        Declaration {
            digest: Some(DigestSource::ReleaseChecksums(&checksums)),
            vendor_team: Some("TEAMID0001"),
        },
    )
    .expect("the digest matches");
    let out = staging(temp.path(), "out");
    let staged = extract_executable(
        &artifact,
        ArtifactShape::TarGz {
            member: "tool-2.0.0/tool",
        },
        &out,
        "tool",
        limits(),
    )
    .expect("the member extracts");
    let installable = verify_signature(staged, &Vendor).expect("the team verifies");
    assert_eq!(installable.path(), out.join("tool"));
    assert_eq!(installable.vendor_team(), Some("TEAMID0001"));
}

#[test]
fn a_refusal_after_the_member_is_written_leaves_no_executable_behind() {
    // The member is written before a later link entry refuses the archive.
    let temp = tempfile::tempdir().unwrap();
    let archive = tar_gz(&[
        file("tool"),
        TarEntry::Symlink {
            name: "link",
            target: "tool",
        },
    ]);
    let artifact = verified(temp.path(), "tool.tar.gz", &archive);
    let out = staging(temp.path(), "out");
    assert!(matches!(
        refused(extract_executable(
            &artifact,
            ArtifactShape::TarGz { member: "tool" },
            &out,
            "tool",
            limits(),
        )),
        Refusal::LinkEntry { .. }
    ));
    assert!(
        !out.join("tool").exists(),
        "a refused executable is removed"
    );
    assert_eq!(std::fs::read_dir(&out).unwrap().count(), 0);
}

#[test]
fn a_zip_refusal_after_the_member_is_written_leaves_no_executable_behind() {
    const S_IFIFO: u32 = 0o010000;
    let temp = tempfile::tempdir().unwrap();
    let archive = zip_of(&[
        ZipEntry::File {
            name: "tool",
            bytes: BINARY.to_vec(),
        },
        ZipEntry::Mode {
            name: "pipe",
            bytes: Vec::new(),
            mode: S_IFIFO | 0o644,
        },
    ]);
    let artifact = verified(temp.path(), "tool.zip", &archive);
    let out = staging(temp.path(), "out");
    assert!(matches!(
        refused(extract_executable(
            &artifact,
            ArtifactShape::Zip { member: "tool" },
            &out,
            "tool",
            limits(),
        )),
        Refusal::UnsupportedEntry { .. }
    ));
    assert!(
        !out.join("tool").exists(),
        "a refused executable is removed"
    );
    assert_eq!(std::fs::read_dir(&out).unwrap().count(), 0);
}

/// Hold sealing at a FIFO so a competing writer wins the destination after
/// extraction starts but before create_new. No scheduling race is needed.
#[test]
#[cfg(unix)]
fn a_file_created_during_sealing_is_not_removed_by_the_losing_extraction() {
    use std::{
        fs::OpenOptions,
        io::Write,
        process::Command,
        thread,
        time::{Duration, Instant},
    };

    let temp = tempfile::tempdir().unwrap();
    let artifact = verified(temp.path(), "artifact.bin", BINARY);
    let source = temp.path().join("artifact.bin");
    std::fs::remove_file(&source).unwrap();
    assert!(
        Command::new("/usr/bin/mkfifo")
            .arg(&source)
            .status()
            .unwrap()
            .success()
    );
    // Opening both ends keeps fixture setup nonblocking. The extractor gets
    // EOF when this handle closes, after it has consumed the supplied bytes.
    let mut pipe = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&source)
        .unwrap();
    let out = staging(temp.path(), "out");
    let sealed = out.join(".tool.artifact");
    let destination = out.join("tool");

    thread::scope(|scope| {
        let extraction = scope
            .spawn(|| extract_executable(&artifact, ArtifactShape::Raw, &out, "tool", limits()));
        let deadline = Instant::now() + Duration::from_secs(5);
        while !sealed.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(1));
        }
        assert!(sealed.exists(), "extraction reached sealing");
        let mut winner = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .unwrap();
        winner.write_all(b"winner").unwrap();
        drop(winner);
        pipe.write_all(BINARY).unwrap();
        while std::fs::metadata(&sealed).unwrap().len() != BINARY.len() as u64
            && Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            std::fs::metadata(&sealed).unwrap().len(),
            BINARY.len() as u64
        );
        drop(pipe);
        let error = extraction.join().unwrap().unwrap_err();
        assert!(
            matches!(error, ProvisionError::Fault(Fault::Io { .. })),
            "{error}"
        );
    });
    assert_eq!(std::fs::read(&destination).unwrap(), b"winner");
    assert!(!sealed.exists(), "the losing call removes its private copy");
}
