// rusteria/src/textures/patterns.rs
//! Global pattern bank: compute-once tileable textures stored as `TexStorage`.
//! Call `ensure_patterns_initialized()` once at startup, or any accessor will lazily init.

use super::TexStorage;
use once_cell::sync::OnceCell;
use rust_embed::RustEmbed;
use std::path::Path;
use strum::EnumCount;

/// Global storage of precomputed patterns.
static PATTERNS: OnceCell<Vec<TexStorage>> = OnceCell::new();

/// Enum of all available patterns, matches the build order in `build_patterns()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumCount)]
pub enum PatternKind {
    Value,
    FbmValue,
}

impl PatternKind {
    pub fn to_index(self) -> usize {
        match self {
            PatternKind::Value => 0,
            PatternKind::FbmValue => 1,
        }
    }

    pub fn from_index(i: usize) -> Option<Self> {
        Some(match i {
            0 => PatternKind::Value,
            1 => PatternKind::FbmValue,
            _ => return None,
        })
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "value" => Some(PatternKind::Value),
            "fbm_value" => Some(PatternKind::FbmValue),
            _ => None,
        }
    }
}

/// Returns true if patterns have already been computed and stored.
#[inline]
pub fn patterns_computed() -> bool {
    PATTERNS.get().is_some()
}

/// Ensure the global patterns vector is initialized. Safe to call multiple times.
pub fn ensure_patterns_initialized() {
    let _ = PATTERNS.get_or_init(|| build_patterns());
}

/// Get an immutable slice of all precomputed patterns. Lazily initializes on first call.
#[inline]
pub fn patterns() -> &'static [TexStorage] {
    PATTERNS.get_or_init(|| build_patterns()).as_slice()
}

/// Get a specific pattern by id. Panics if out of range.
#[inline]
pub fn pattern(id: usize) -> &'static TexStorage {
    let vec = PATTERNS.get_or_init(|| build_patterns());
    &vec[id]
}

/// Get a specific pattern by id.
pub fn pattern_safe(id: usize) -> Option<&'static TexStorage> {
    let vec = PATTERNS.get_or_init(|| build_patterns());
    vec.get(id)
}

#[derive(RustEmbed)]
#[folder = "embedded/"]
struct EmbeddedPatterns;

fn build_patterns() -> Vec<TexStorage> {
    const PATTERN_COUNT: usize = PatternKind::COUNT;
    let mut out: Vec<TexStorage> = (0..PATTERN_COUNT).map(|_| TexStorage::new(1, 1)).collect();

    for file in EmbeddedPatterns::iter() {
        let path_str = file.as_ref();
        if !path_str.ends_with(".png") {
            continue;
        }
        let stem = Path::new(path_str)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if let Some(kind) = PatternKind::from_name(&stem) {
            if let Some(bytes) = EmbeddedPatterns::get(path_str) {
                if let Ok(tex) = TexStorage::from_png_bytes(bytes.data.as_ref()) {
                    out[kind.to_index()] = tex;
                }
            }
        }
    }

    out
}
