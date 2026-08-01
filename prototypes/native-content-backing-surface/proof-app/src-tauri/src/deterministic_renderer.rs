use longhorn_core::{PhysicalPoint, PhysicalRect};
use serde::Serialize;

const LIT: u8 = 199;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct PixelEvidence {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) lit_pixels: u64,
    pub(crate) outside_clip_lit_pixels: u64,
    pub(crate) digest: String,
}

#[derive(Clone, Default)]
pub(crate) struct DeterministicRenderer {
    sequence: u64,
}

impl DeterministicRenderer {
    pub(crate) fn render(
        &mut self,
        storage: PhysicalRect,
        clip: PhysicalRect,
        presentation_enabled: bool,
    ) -> (u64, PixelEvidence) {
        self.sequence += 1;
        let mut digest = 0xcbf2_9ce4_8422_2325_u64;
        let mut lit_pixels = 0_u64;
        let mut outside_clip_lit_pixels = 0_u64;
        for y in 0..storage.size().height() {
            for x in 0..storage.size().width() {
                let absolute = PhysicalPoint::new(
                    storage
                        .origin()
                        .x()
                        .get()
                        .saturating_add(i32::try_from(x).unwrap_or(i32::MAX)),
                    storage
                        .origin()
                        .y()
                        .get()
                        .saturating_add(i32::try_from(y).unwrap_or(i32::MAX)),
                );
                let inside = clip.contains_point(&absolute);
                let pixel = if presentation_enabled && inside {
                    LIT
                } else {
                    0
                };
                if pixel != 0 {
                    lit_pixels += 1;
                    if !inside {
                        outside_clip_lit_pixels += 1;
                    }
                }
                digest ^= u64::from(pixel);
                digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        (
            self.sequence,
            PixelEvidence {
                width: storage.size().width(),
                height: storage.size().height(),
                lit_pixels,
                outside_clip_lit_pixels,
                digest: format!("{digest:016x}"),
            },
        )
    }
}
