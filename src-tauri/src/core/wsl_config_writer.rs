//! Write-side counterpart to `core::health_wsl`. Reads the `[wsl2]`
//! section of a `.wslconfig` INI body, applies user-requested overrides,
//! and renders a new body that preserves comments, unknown keys, and
//! other sections. The render function is pure and exhaustively tested.
//!
//! Used by A2 of the backend roadmap to opt-in mirrored networking and
//! sane defaults (`vmIdleTimeout=-1`, `nestedVirtualization=true`)
//! without clobbering whatever the user already had in there.

use serde::{Deserialize, Serialize};

/// Overrides to apply to the `[wsl2]` section of `.wslconfig`. Any field
/// set to `Some(_)` is written; `None` leaves the existing value alone.
/// Strings are not validated — the caller is responsible for passing
/// well-formed values (e.g. `"mirrored"`, `"NAT"`, `"16GB"`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WslConfigOverrides {
    pub vm_idle_timeout_ms: Option<i64>,
    pub memory: Option<String>,
    pub swap: Option<String>,
    pub processors: Option<u32>,
    pub nested_virtualization: Option<bool>,
    pub networking_mode: Option<String>,
}

impl WslConfigOverrides {
    /// Convenience: preset that addresses the friction we hit in the
    /// spike session — no idle shutdown, nested virt on, mirrored
    /// networking when available.
    pub fn winbox_recommended() -> Self {
        Self {
            vm_idle_timeout_ms: Some(-1),
            nested_virtualization: Some(true),
            networking_mode: Some("mirrored".into()),
            ..Default::default()
        }
    }
}

/// Apply the overrides on top of the existing `.wslconfig` body and
/// return the new body. Pure function. Behavior:
///
/// * If `[wsl2]` exists, we patch it in place: each override either
///   replaces an existing matching key or is appended at the end of
///   the section.
/// * If `[wsl2]` is absent, it's appended (with a blank line spacer).
/// * Other sections, comments, and unknown `[wsl2]` keys are kept as-is.
/// * A trailing newline is guaranteed.
pub fn render_wslconfig_with(existing: &str, overrides: &WslConfigOverrides) -> String {
    // Compute every key=value pair we have to ensure exists in [wsl2].
    let mut wanted: Vec<(&'static str, String)> = Vec::new();
    if let Some(v) = overrides.vm_idle_timeout_ms {
        wanted.push(("vmIdleTimeout", v.to_string()));
    }
    if let Some(v) = &overrides.memory {
        wanted.push(("memory", v.clone()));
    }
    if let Some(v) = &overrides.swap {
        wanted.push(("swap", v.clone()));
    }
    if let Some(v) = overrides.processors {
        wanted.push(("processors", v.to_string()));
    }
    if let Some(v) = overrides.nested_virtualization {
        wanted.push(("nestedVirtualization", v.to_string()));
    }
    if let Some(v) = &overrides.networking_mode {
        wanted.push(("networkingMode", v.clone()));
    }
    if wanted.is_empty() {
        // Normalize trailing newline regardless.
        return ensure_trailing_newline(existing);
    }

    // First pass: copy the existing body, replacing any line in the
    // [wsl2] section whose key matches one we want. Track which wanted
    // keys we've consumed so we can append the rest.
    let mut out: Vec<String> = Vec::new();
    let mut in_wsl2 = false;
    let mut wsl2_seen = false;
    let mut consumed: Vec<bool> = vec![false; wanted.len()];
    let mut wsl2_section_end_idx: Option<usize> = None;

    for raw in existing.lines() {
        let trimmed = raw.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Leaving a section: if we were in [wsl2], remember where
            // it ends so we can append leftover overrides there.
            if in_wsl2 {
                wsl2_section_end_idx = Some(out.len());
            }
            in_wsl2 = trimmed.eq_ignore_ascii_case("[wsl2]");
            if in_wsl2 {
                wsl2_seen = true;
            }
            out.push(raw.to_string());
            continue;
        }
        if in_wsl2 {
            if let Some((key, _)) = trimmed.split_once('=') {
                let lower = key.trim().to_lowercase();
                if let Some(pos) = wanted.iter().position(|(k, _)| k.to_lowercase() == lower) {
                    let (k, v) = &wanted[pos];
                    out.push(format!("{}={}", k, v));
                    consumed[pos] = true;
                    continue;
                }
            }
        }
        out.push(raw.to_string());
    }

    // If we ended the file still inside [wsl2], its end is "the very
    // end of `out`".
    if in_wsl2 && wsl2_section_end_idx.is_none() {
        wsl2_section_end_idx = Some(out.len());
    }

    // Append leftover (un-consumed) overrides into [wsl2].
    let leftovers: Vec<String> = wanted
        .iter()
        .enumerate()
        .filter_map(|(i, (k, v))| {
            if consumed[i] {
                None
            } else {
                Some(format!("{}={}", k, v))
            }
        })
        .collect();

    if !leftovers.is_empty() {
        if wsl2_seen {
            let insert_at = wsl2_section_end_idx.unwrap_or(out.len());
            // Walk back over blank lines so we insert immediately after
            // the last non-blank line of [wsl2] instead of after blanks.
            let mut at = insert_at;
            while at > 0 && out[at - 1].trim().is_empty() {
                at -= 1;
            }
            for (offset, line) in leftovers.into_iter().enumerate() {
                out.insert(at + offset, line);
            }
        } else {
            // No [wsl2] section at all — append one.
            if !out.is_empty() && !out.last().map(|l| l.is_empty()).unwrap_or(false) {
                out.push(String::new());
            }
            out.push("[wsl2]".into());
            for line in leftovers {
                out.push(line);
            }
        }
    }

    ensure_trailing_newline(&out.join("\n"))
}

fn ensure_trailing_newline(s: &str) -> String {
    if s.ends_with('\n') {
        s.to_string()
    } else {
        format!("{s}\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_overrides_leaves_body_alone() {
        let existing = "[wsl2]\nmemory=8GB\n";
        let body = render_wslconfig_with(existing, &WslConfigOverrides::default());
        assert_eq!(body, existing);
    }

    #[test]
    fn append_wsl2_section_when_missing() {
        let existing = "[user]\ndefault=bruno\n";
        let body = render_wslconfig_with(
            existing,
            &WslConfigOverrides {
                nested_virtualization: Some(true),
                ..Default::default()
            },
        );
        assert!(body.contains("[user]"));
        assert!(body.contains("default=bruno"));
        assert!(body.contains("[wsl2]"));
        assert!(body.contains("nestedVirtualization=true"));
    }

    #[test]
    fn replace_existing_key_in_place_preserves_others() {
        let existing = "[wsl2]\nmemory=8GB\nnestedVirtualization=false\nprocessors=4\n";
        let body = render_wslconfig_with(
            existing,
            &WslConfigOverrides {
                nested_virtualization: Some(true),
                ..Default::default()
            },
        );
        assert!(body.contains("memory=8GB"));
        assert!(body.contains("processors=4"));
        assert!(body.contains("nestedVirtualization=true"));
        assert!(!body.contains("nestedVirtualization=false"));
    }

    #[test]
    fn append_new_key_into_existing_wsl2_section() {
        let existing = "[wsl2]\nmemory=8GB\n\n[experimental]\nfoo=bar\n";
        let body = render_wslconfig_with(
            existing,
            &WslConfigOverrides {
                networking_mode: Some("mirrored".into()),
                ..Default::default()
            },
        );
        // networkingMode must land inside [wsl2], BEFORE [experimental].
        let net_idx = body
            .find("networkingMode=mirrored")
            .expect("networking present");
        let exp_idx = body.find("[experimental]").expect("experimental present");
        assert!(
            net_idx < exp_idx,
            "networking key landed after wrong section"
        );
        assert!(body.contains("foo=bar"));
    }

    #[test]
    fn case_insensitive_key_match_replaces_in_place() {
        let existing = "[wsl2]\nMEMORY=4GB\n";
        let body = render_wslconfig_with(
            existing,
            &WslConfigOverrides {
                memory: Some("16GB".into()),
                ..Default::default()
            },
        );
        // Original casing of the key is replaced; new line uses our casing.
        assert!(body.contains("memory=16GB"));
        assert!(!body.contains("4GB"));
    }

    #[test]
    fn vm_idle_timeout_minus_one_renders_correctly() {
        let body = render_wslconfig_with(
            "",
            &WslConfigOverrides {
                vm_idle_timeout_ms: Some(-1),
                ..Default::default()
            },
        );
        assert!(body.contains("[wsl2]"));
        assert!(body.contains("vmIdleTimeout=-1"));
    }

    #[test]
    fn winbox_recommended_writes_three_keys() {
        let body = render_wslconfig_with("", &WslConfigOverrides::winbox_recommended());
        assert!(body.contains("vmIdleTimeout=-1"));
        assert!(body.contains("nestedVirtualization=true"));
        assert!(body.contains("networkingMode=mirrored"));
    }

    #[test]
    fn idempotent_when_applied_twice() {
        let first = render_wslconfig_with("", &WslConfigOverrides::winbox_recommended());
        let second = render_wslconfig_with(&first, &WslConfigOverrides::winbox_recommended());
        assert_eq!(first, second);
    }

    #[test]
    fn preserves_inline_comments_on_unrelated_keys() {
        let existing = "[wsl2]\nmemory=8GB # bumped for build\nswap=2GB\n";
        let body = render_wslconfig_with(
            existing,
            &WslConfigOverrides {
                networking_mode: Some("mirrored".into()),
                ..Default::default()
            },
        );
        assert!(body.contains("memory=8GB # bumped for build"));
        assert!(body.contains("swap=2GB"));
        assert!(body.contains("networkingMode=mirrored"));
    }

    #[test]
    fn ends_with_trailing_newline() {
        let body = render_wslconfig_with(
            "[wsl2]\nmemory=4GB",
            &WslConfigOverrides {
                memory: Some("8GB".into()),
                ..Default::default()
            },
        );
        assert!(body.ends_with('\n'));
    }
}
