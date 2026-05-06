//! Design tokens — formalises the v3.1 UX deep-dive (Track B §5.1) into
//! named constants the rest of the GUI can reference instead of hard-coded
//! pixel values.
//!
//! These are **tokens**, not theme colours. Colours live in `theme.rs` and
//! are dark/light-mode dependent. Spacing, motion, typography, and elevation
//! are mode-independent and live here.
//!
//! Naming follows the pattern `{kind}_{semantic}` so callers read like
//! `tokens::SPACE_MD` rather than `tokens::EIGHT`. If you find yourself
//! reaching for a literal `8.0` somewhere in the GUI, add a token here
//! instead.
//!
//! Reference: `docs/COMMERCIAL_CAD_ROADMAP.md` §5.1 design system.

#![allow(dead_code)]

use std::time::Duration;

// ---------------------------------------------------------------------------
// Spacing — 8 px base grid
// ---------------------------------------------------------------------------

/// 0 px — used for "no margin" frames where `0.0` is more readable as a token.
pub const SPACE_NONE: f32 = 0.0;
/// 2 px — hairline padding (icon-only buttons).
pub const SPACE_2XS: f32 = 2.0;
/// 4 px — tight padding inside dense rows.
pub const SPACE_XS: f32 = 4.0;
/// 8 px — base grid step. Default item spacing.
pub const SPACE_SM: f32 = 8.0;
/// 12 px — comfortable padding inside cards / dialog bodies.
pub const SPACE_MD: f32 = 12.0;
/// 16 px — section separation.
pub const SPACE_LG: f32 = 16.0;
/// 24 px — page-level breathing room.
pub const SPACE_XL: f32 = 24.0;
/// 32 px — dialog outer margin.
pub const SPACE_2XL: f32 = 32.0;

// ---------------------------------------------------------------------------
// Hit targets — minimum sizes for clickable areas (accessibility)
// ---------------------------------------------------------------------------

/// Minimum hit-target side length per WCAG 2.5.5 (24 px AA, 44 px AAA).
/// We default to 28 px which clears AA for desktop pointer input while
/// keeping the dense CAD layout we want.
pub const HIT_TARGET_MIN: f32 = 28.0;
/// Touch hit target — used when input device is detected as touch / pen.
pub const HIT_TARGET_TOUCH: f32 = 44.0;

// ---------------------------------------------------------------------------
// Typography — modular scale (1.125 ratio rounded)
// ---------------------------------------------------------------------------

pub const FONT_2XS: f32 = 9.0;
pub const FONT_XS: f32 = 10.0;
pub const FONT_SM: f32 = 11.0;
pub const FONT_MD: f32 = 13.0;
pub const FONT_LG: f32 = 15.0;
pub const FONT_XL: f32 = 18.0;
pub const FONT_2XL: f32 = 22.0;

// ---------------------------------------------------------------------------
// Border radius
// ---------------------------------------------------------------------------

pub const RADIUS_NONE: f32 = 0.0;
pub const RADIUS_SM: f32 = 2.0;
pub const RADIUS_MD: f32 = 4.0;
pub const RADIUS_LG: f32 = 8.0;
pub const RADIUS_PILL: f32 = 9999.0;

// ---------------------------------------------------------------------------
// Stroke widths
// ---------------------------------------------------------------------------

pub const STROKE_HAIRLINE: f32 = 0.5;
pub const STROKE_THIN: f32 = 1.0;
pub const STROKE_REGULAR: f32 = 1.5;
pub const STROKE_BOLD: f32 = 2.5;

// ---------------------------------------------------------------------------
// Elevation — drop-shadow blur radius for floating surfaces
// ---------------------------------------------------------------------------

pub const ELEVATION_RAISED: f32 = 4.0;   // toolbar, dock header
pub const ELEVATION_OVERLAY: f32 = 12.0; // dropdown, tooltip
pub const ELEVATION_DIALOG: f32 = 24.0;  // modal
pub const ELEVATION_TOAST: f32 = 16.0;

// ---------------------------------------------------------------------------
// Motion — animation durations
// ---------------------------------------------------------------------------

/// Instantaneous — no animation perceptible (snap focus, selection commit).
pub const DURATION_INSTANT: Duration = Duration::from_millis(0);
/// 100 ms — micro-interactions: hover, press feedback. Per Doherty threshold,
/// the perception of "instant" tops out around 100 ms.
pub const DURATION_FAST: Duration = Duration::from_millis(100);
/// 180 ms — UI transitions: dropdown open, panel reveal.
pub const DURATION_BASE: Duration = Duration::from_millis(180);
/// 250 ms — modal appearance, dock undock.
pub const DURATION_SLOW: Duration = Duration::from_millis(250);
/// 400 ms — emphasis (welcome screen fade, first-run hints).
pub const DURATION_DELIBERATE: Duration = Duration::from_millis(400);

/// Toast auto-dismiss timing.
pub const TOAST_DISMISS_INFO: Duration = Duration::from_secs(4);
pub const TOAST_DISMISS_SUCCESS: Duration = Duration::from_secs(3);
pub const TOAST_DISMISS_WARNING: Duration = Duration::from_secs(6);
pub const TOAST_DISMISS_ERROR: Duration = Duration::from_secs(8);

// ---------------------------------------------------------------------------
// Z-order — for stacking overlays
// ---------------------------------------------------------------------------

pub const Z_BACKGROUND: i32 = 0;
pub const Z_PANEL: i32 = 100;
pub const Z_OVERLAY: i32 = 500;
pub const Z_TOAST: i32 = 800;
pub const Z_DIALOG: i32 = 900;
pub const Z_PALETTE: i32 = 950;
pub const Z_TOOLTIP: i32 = 1000;
