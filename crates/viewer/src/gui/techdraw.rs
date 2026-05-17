//! TechDraw workbench actions.
//!
//! Extracted from `gui/mod.rs` as part of the viewer module split. Holds the
//! TechDraw drawing-sheet operations (page management, view projection,
//! dimensions, annotations, centerlines, exports) previously inlined as
//! top-level `GuiAction` variants.
//!
//! Note: `TechDrawAction` derives only `Clone + Debug` because the
//! `ExportSvg` / `ExportDxf` / `ExportPdf` variants carry `PathBuf`, and
//! `cadkernel_io::ProjectionDir` lacks `PartialEq` derives in some configs.

use std::fmt::Write;
use std::path::PathBuf;

use cadkernel_core::{KernelError, KernelResult};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GdtSymbol {
    Straightness,
    Flatness,
    Circularity,
    Cylindricity,
    ProfileLine,
    ProfileSurface,
    Angularity,
    Perpendicularity,
    Parallelism,
    Position,
    Concentricity,
    Symmetry,
    Runout,
    TotalRunout,
}

#[allow(dead_code)]
impl GdtSymbol {
    pub const ALL: [Self; 14] = [
        Self::Straightness,
        Self::Flatness,
        Self::Circularity,
        Self::Cylindricity,
        Self::ProfileLine,
        Self::ProfileSurface,
        Self::Angularity,
        Self::Perpendicularity,
        Self::Parallelism,
        Self::Position,
        Self::Concentricity,
        Self::Symmetry,
        Self::Runout,
        Self::TotalRunout,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Straightness => "Straightness",
            Self::Flatness => "Flatness",
            Self::Circularity => "Circularity",
            Self::Cylindricity => "Cylindricity",
            Self::ProfileLine => "Profile of a Line",
            Self::ProfileSurface => "Profile of a Surface",
            Self::Angularity => "Angularity",
            Self::Perpendicularity => "Perpendicularity",
            Self::Parallelism => "Parallelism",
            Self::Position => "Position",
            Self::Concentricity => "Concentricity",
            Self::Symmetry => "Symmetry",
            Self::Runout => "Circular Runout",
            Self::TotalRunout => "Total Runout",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Self::Straightness => "⏤",
            Self::Flatness => "▱",
            Self::Circularity => "○",
            Self::Cylindricity => "⌭",
            Self::ProfileLine => "⌒",
            Self::ProfileSurface => "⌓",
            Self::Angularity => "∠",
            Self::Perpendicularity => "⟂",
            Self::Parallelism => "∥",
            Self::Position => "⌖",
            Self::Concentricity => "◎",
            Self::Symmetry => "⌯",
            Self::Runout => "↗",
            Self::TotalRunout => "⌰",
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DatumLabel(String);

#[allow(dead_code)]
impl DatumLabel {
    pub fn new(label: impl Into<String>) -> KernelResult<Self> {
        let label = label.into();
        let valid = !label.is_empty()
            && label.len() <= 3
            && label.chars().all(|ch| ch.is_ascii_uppercase());
        if !valid {
            return Err(KernelError::InvalidArgument(format!(
                "datum label must be 1-3 uppercase ASCII letters: {label:?}"
            )));
        }
        Ok(Self(label))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn bracketed(&self) -> String {
        format!("[{}]", self.0)
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DatumFeature {
    pub label: DatumLabel,
    pub x: f64,
    pub y: f64,
}

#[allow(dead_code)]
impl DatumFeature {
    pub fn new(label: DatumLabel, x: f64, y: f64) -> KernelResult<Self> {
        validate_finite("datum x", x)?;
        validate_finite("datum y", y)?;
        Ok(Self { label, x, y })
    }

    pub fn to_svg(&self) -> String {
        let text = escape_svg_text(self.label.as_str());
        format!(
            "<g class=\"techdraw-datum\"><rect x=\"{}\" y=\"{}\" width=\"8\" height=\"8\" fill=\"white\" stroke=\"black\" stroke-width=\"0.35\"/><text x=\"{}\" y=\"{}\" font-size=\"5\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"black\">{}</text></g>",
            self.x - 4.0,
            self.y - 4.0,
            self.x,
            self.y,
            text
        )
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GdtFrame {
    pub characteristic: GdtSymbol,
    pub tolerance: f64,
    pub datum_refs: Vec<DatumLabel>,
}

#[allow(dead_code)]
impl GdtFrame {
    pub fn new(
        characteristic: GdtSymbol,
        tolerance: f64,
        datum_refs: Vec<DatumLabel>,
    ) -> KernelResult<Self> {
        validate_non_negative_finite("GD&T tolerance", tolerance)?;
        if datum_refs.len() > 3 {
            return Err(KernelError::InvalidArgument(format!(
                "GD&T frame supports at most 3 datum references, got {}",
                datum_refs.len()
            )));
        }
        Ok(Self {
            characteristic,
            tolerance,
            datum_refs,
        })
    }

    pub fn control_text(&self) -> String {
        let mut text = format!("{} {:.3}", self.characteristic.glyph(), self.tolerance);
        for datum in &self.datum_refs {
            text.push(' ');
            text.push_str(&datum.bracketed());
        }
        text
    }

    pub fn to_text_annotation(
        &self,
        x: f64,
        y: f64,
        font_size: f64,
    ) -> cadkernel_io::TextAnnotation {
        cadkernel_io::TextAnnotation {
            position: cadkernel_math::Point2::new(x, y),
            text: self.control_text(),
            font_size,
        }
    }

    pub fn apply_to_sheet(
        &self,
        sheet: &mut cadkernel_io::DrawingSheet,
        x: f64,
        y: f64,
        font_size: f64,
    ) {
        sheet
            .text_annotations
            .push(self.to_text_annotation(x, y, font_size));
    }

    pub fn to_svg_at(&self, x: f64, y: f64) -> String {
        let mut out = String::new();
        let cells = self.svg_cells();
        let cell_height = 8.0;
        let total_width: f64 = cells.iter().map(|(_, width)| *width).sum();
        let _ = write!(
            out,
            "<g class=\"techdraw-gdt-frame\" data-characteristic=\"{}\">",
            escape_svg_text(self.characteristic.label())
        );
        let _ = write!(
            out,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"white\" stroke=\"black\" stroke-width=\"0.35\"/>",
            x, y, total_width, cell_height
        );
        let mut cursor = x;
        for (idx, (label, width)) in cells.iter().enumerate() {
            if idx > 0 {
                let _ = write!(
                    out,
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"0.25\"/>",
                    cursor,
                    y,
                    cursor,
                    y + cell_height
                );
            }
            let _ = write!(
                out,
                "<text x=\"{}\" y=\"{}\" font-size=\"4.2\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"black\">{}</text>",
                cursor + width * 0.5,
                y + cell_height * 0.55,
                escape_svg_text(label)
            );
            cursor += width;
        }
        out.push_str("</g>");
        out
    }

    fn svg_cells(&self) -> Vec<(String, f64)> {
        let mut cells = vec![
            (self.characteristic.glyph().to_string(), 8.0),
            (format!("{:.3}", self.tolerance), 16.0),
        ];
        for datum in &self.datum_refs {
            cells.push((datum.as_str().to_string(), 8.0));
        }
        cells
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BomEntry {
    pub item_no: u32,
    pub qty: u32,
    pub description: String,
    pub material: String,
}

#[allow(dead_code)]
impl BomEntry {
    pub fn new(
        item_no: u32,
        qty: u32,
        description: impl Into<String>,
        material: impl Into<String>,
    ) -> KernelResult<Self> {
        if item_no == 0 {
            return Err(KernelError::InvalidArgument(
                "BOM item number must be positive".into(),
            ));
        }
        if qty == 0 {
            return Err(KernelError::InvalidArgument(
                "BOM quantity must be positive".into(),
            ));
        }
        let description = description.into();
        if description.trim().is_empty() {
            return Err(KernelError::InvalidArgument(
                "BOM description must not be empty".into(),
            ));
        }
        let material = normalize_material(material.into());
        Ok(Self {
            item_no,
            qty,
            description,
            material,
        })
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BomTableLayout {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub row_height: f64,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Bom {
    pub entries: Vec<BomEntry>,
}

#[allow(dead_code)]
impl Bom {
    pub fn new(entries: Vec<BomEntry>) -> KernelResult<Self> {
        let mut bom = Self { entries };
        bom.renumber()?;
        Ok(bom)
    }

    pub fn from_parts<I, D, M>(parts: I) -> Self
    where
        I: IntoIterator<Item = (D, M)>,
        D: Into<String>,
        M: Into<String>,
    {
        let mut grouped = std::collections::BTreeMap::<(String, String), u32>::new();
        for (description, material) in parts {
            let description = description.into();
            if description.trim().is_empty() {
                continue;
            }
            let material = normalize_material(material.into());
            *grouped.entry((description, material)).or_insert(0) += 1;
        }

        let entries = grouped
            .into_iter()
            .enumerate()
            .map(|(index, ((description, material), qty))| BomEntry {
                item_no: index as u32 + 1,
                qty,
                description,
                material,
            })
            .collect();
        Self { entries }
    }

    pub fn push_manual(
        &mut self,
        qty: u32,
        description: impl Into<String>,
        material: impl Into<String>,
    ) -> KernelResult<()> {
        let item_no = self.entries.len() as u32 + 1;
        self.entries
            .push(BomEntry::new(item_no, qty, description, material)?);
        Ok(())
    }

    pub fn layout_bottom_right(&self, sheet_width: f64, sheet_height: f64) -> BomTableLayout {
        let row_height = 7.0;
        let width = 92.0;
        let height = row_height * (self.entries.len() as f64 + 1.0);
        let margin = 12.0;
        BomTableLayout {
            x: (sheet_width - width - margin).max(margin),
            y: (sheet_height - height - margin).max(margin),
            width,
            height,
            row_height,
        }
    }

    pub fn to_text_annotations(
        &self,
        layout: BomTableLayout,
        font_size: f64,
    ) -> Vec<cadkernel_io::TextAnnotation> {
        let mut annotations = Vec::with_capacity(self.entries.len() + 1);
        annotations.push(cadkernel_io::TextAnnotation {
            position: cadkernel_math::Point2::new(layout.x + 2.0, layout.y + font_size),
            text: "ITEM QTY DESCRIPTION MATERIAL".into(),
            font_size,
        });
        for (idx, entry) in self.entries.iter().enumerate() {
            annotations.push(cadkernel_io::TextAnnotation {
                position: cadkernel_math::Point2::new(
                    layout.x + 2.0,
                    layout.y + layout.row_height * (idx as f64 + 1.0) + font_size,
                ),
                text: format!(
                    "{} {} {} {}",
                    entry.item_no, entry.qty, entry.description, entry.material
                ),
                font_size,
            });
        }
        annotations
    }

    pub fn apply_to_sheet(&self, sheet: &mut cadkernel_io::DrawingSheet, font_size: f64) {
        let layout = self.layout_bottom_right(sheet.width, sheet.height);
        sheet
            .text_annotations
            .extend(self.to_text_annotations(layout, font_size));
    }

    pub fn to_svg_bottom_right(&self, sheet_width: f64, sheet_height: f64) -> String {
        self.to_svg_at(self.layout_bottom_right(sheet_width, sheet_height))
    }

    pub fn to_svg_at(&self, layout: BomTableLayout) -> String {
        let mut out = String::new();
        let _ = write!(
            out,
            "<g class=\"techdraw-bom\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"white\" stroke=\"black\" stroke-width=\"0.35\"/>",
            layout.x, layout.y, layout.width, layout.height
        );
        let cols = [10.0, 12.0, 48.0, 22.0];
        let mut cursor = layout.x;
        for width in cols.iter().take(cols.len() - 1) {
            cursor += width;
            let _ = write!(
                out,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"0.25\"/>",
                cursor,
                layout.y,
                cursor,
                layout.y + layout.height
            );
        }
        for row in 1..=self.entries.len() {
            let y = layout.y + layout.row_height * row as f64;
            let _ = write!(
                out,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"0.25\"/>",
                layout.x,
                y,
                layout.x + layout.width,
                y
            );
        }
        write_bom_row_svg(
            &mut out,
            layout.x,
            layout.y,
            layout.row_height,
            &cols,
            ["ITEM", "QTY", "DESCRIPTION", "MATERIAL"],
        );
        for (idx, entry) in self.entries.iter().enumerate() {
            let y = layout.y + layout.row_height * (idx as f64 + 1.0);
            let item_no = entry.item_no.to_string();
            let qty = entry.qty.to_string();
            write_bom_row_svg(
                &mut out,
                layout.x,
                y,
                layout.row_height,
                &cols,
                [
                    item_no.as_str(),
                    qty.as_str(),
                    &entry.description,
                    &entry.material,
                ],
            );
        }
        out.push_str("</g>");
        out
    }

    pub fn to_csv(&self) -> String {
        let rows: Vec<_> = self
            .entries
            .iter()
            .map(|entry| cadkernel_io::techdraw_dxf::TechDrawBomCsvRow {
                item_no: entry.item_no,
                qty: entry.qty,
                description: entry.description.clone(),
                material: entry.material.clone(),
            })
            .collect();
        cadkernel_io::techdraw_dxf::techdraw_bom_to_csv(&rows)
    }

    fn renumber(&mut self) -> KernelResult<()> {
        for (idx, entry) in self.entries.iter_mut().enumerate() {
            if entry.qty == 0 {
                return Err(KernelError::InvalidArgument(
                    "BOM quantity must be positive".into(),
                ));
            }
            if entry.description.trim().is_empty() {
                return Err(KernelError::InvalidArgument(
                    "BOM description must not be empty".into(),
                ));
            }
            entry.item_no = idx as u32 + 1;
            entry.material = normalize_material(std::mem::take(&mut entry.material));
        }
        Ok(())
    }
}

#[allow(dead_code)]
fn validate_finite(label: &str, value: f64) -> KernelResult<()> {
    if !value.is_finite() {
        return Err(KernelError::InvalidArgument(format!(
            "{label} must be finite"
        )));
    }
    Ok(())
}

#[allow(dead_code)]
fn validate_non_negative_finite(label: &str, value: f64) -> KernelResult<()> {
    validate_finite(label, value)?;
    if value < 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "{label} must be non-negative"
        )));
    }
    Ok(())
}

#[allow(dead_code)]
fn normalize_material(material: String) -> String {
    let trimmed = material.trim();
    if trimmed.is_empty() {
        "Unspecified".into()
    } else {
        trimmed.into()
    }
}

#[allow(dead_code)]
fn escape_svg_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[allow(dead_code)]
fn write_bom_row_svg(
    out: &mut String,
    x: f64,
    y: f64,
    row_height: f64,
    cols: &[f64; 4],
    values: [&str; 4],
) {
    let mut cursor = x;
    for (idx, value) in values.iter().enumerate() {
        let anchor = if idx < 2 { "middle" } else { "start" };
        let tx = if idx < 2 {
            cursor + cols[idx] * 0.5
        } else {
            cursor + 1.5
        };
        let _ = write!(
            out,
            "<text x=\"{}\" y=\"{}\" font-size=\"3.2\" text-anchor=\"{}\" dominant-baseline=\"middle\" fill=\"black\">{}</text>",
            tx,
            y + row_height * 0.55,
            anchor,
            escape_svg_text(value)
        );
        cursor += cols[idx];
    }
}

/// Actions specific to the TechDraw workbench, dispatched through
/// `GuiAction::TechDraw(TechDrawAction)`.
#[derive(Clone, Debug)]
pub(crate) enum TechDrawAction {
    // Page management
    NewPage,
    FromTemplate,
    OpenPageSetup,
    CommitPageSetup,
    Redraw,
    Clear,

    // View projection
    AddView(cadkernel_io::ProjectionDir),
    ThreeView,
    SectionView,
    DetailView,
    BrokenView,
    OpenViewSetup(TechDrawViewKind),
    CommitViewSetup,

    // Dimensions
    DimLinear,
    DimRadius,
    DimDiameter,
    DimAngle,
    DimArcLen,
    DimArea,
    OpenDimensionSetup(TechDrawDimensionKind),
    CommitDimensionSetup,

    // Annotation
    Text,
    RichText,
    Balloon,
    Leader,
    Weld,
    SurfFinish,
    OpenAnnotationSetup(TechDrawAnnotationKind),
    CommitAnnotationSetup,

    // Centerlines / bolt circles
    CenterFace,
    CenterLines,
    CenterPoints,
    BoltCircle,
    OpenCenterlineSetup(TechDrawCenterlineKind),
    CommitCenterlineSetup,

    // Exports
    ExportSvg(PathBuf),
    ExportDxf(PathBuf),
    ExportPdf(PathBuf),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TechDrawViewKind {
    Front,
    Top,
    Right,
    Isometric,
    ThreeView,
    Section,
    Detail,
    Broken,
}

impl TechDrawViewKind {
    pub const ALL: [Self; 8] = [
        Self::Front,
        Self::Top,
        Self::Right,
        Self::Isometric,
        Self::ThreeView,
        Self::Section,
        Self::Detail,
        Self::Broken,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Front => "Front View",
            Self::Top => "Top View",
            Self::Right => "Right View",
            Self::Isometric => "Isometric View",
            Self::ThreeView => "3-View Projection",
            Self::Section => "Section View",
            Self::Detail => "Detail View",
            Self::Broken => "Broken View",
        }
    }

    pub fn projection_dir(self) -> Option<cadkernel_io::ProjectionDir> {
        match self {
            Self::Front => Some(cadkernel_io::ProjectionDir::Front),
            Self::Top => Some(cadkernel_io::ProjectionDir::Top),
            Self::Right => Some(cadkernel_io::ProjectionDir::Right),
            Self::Isometric => Some(cadkernel_io::ProjectionDir::Isometric),
            Self::ThreeView | Self::Section | Self::Detail | Self::Broken => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TechDrawViewSetupState {
    pub kind: TechDrawViewKind,
    pub sheet_x: f64,
    pub sheet_y: f64,
    pub auto_scale: bool,
    pub sheet_scale: f64,
    pub spacing_x: f64,
    pub spacing_y: f64,
    pub detail_center_x: f64,
    pub detail_center_y: f64,
    pub detail_radius: f64,
    pub detail_magnification: f64,
    pub break_start: f64,
    pub break_end: f64,
    pub break_gap: f64,
    pub section_label: String,
}

impl TechDrawViewSetupState {
    pub fn new(kind: TechDrawViewKind, sheet_width: f64, sheet_height: f64) -> Self {
        Self {
            kind,
            sheet_x: sheet_width * 0.50,
            sheet_y: sheet_height * 0.55,
            auto_scale: true,
            sheet_scale: 40.0,
            spacing_x: sheet_width * 0.32,
            spacing_y: sheet_height * 0.32,
            detail_center_x: 0.0,
            detail_center_y: 0.0,
            detail_radius: 20.0,
            detail_magnification: 2.0,
            break_start: 15.0,
            break_end: 45.0,
            break_gap: 5.0,
            section_label: "A-A".into(),
        }
    }

    pub fn apply_kind(&mut self, kind: TechDrawViewKind, sheet_width: f64, sheet_height: f64) {
        let mut next = Self::new(kind, sheet_width, sheet_height);
        next.sheet_x = self.sheet_x;
        next.sheet_y = self.sheet_y;
        next.auto_scale = self.auto_scale;
        next.sheet_scale = self.sheet_scale;
        *self = next;
    }

    pub fn scale_override(&self) -> Option<f64> {
        (!self.auto_scale).then_some(self.sheet_scale.max(1e-6))
    }

    pub fn apply_placement(&self, view: &mut cadkernel_io::DrawingView) {
        view.set_sheet_placement(self.sheet_x, self.sheet_y, self.scale_override());
    }

    pub fn apply_three_view_placement(&self, view: &mut cadkernel_io::DrawingView, index: usize) {
        let (x, y) = match index {
            0 => (self.sheet_x, self.sheet_y),
            1 => (self.sheet_x, self.sheet_y - self.spacing_y),
            2 => (self.sheet_x + self.spacing_x, self.sheet_y),
            _ => (self.sheet_x, self.sheet_y),
        };
        view.set_sheet_placement(x, y, self.scale_override());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TechDrawDimensionKind {
    Linear,
    Radius,
    Diameter,
    Angle,
    ArcLength,
    Area,
}

impl TechDrawDimensionKind {
    pub const ALL: [Self; 6] = [
        Self::Linear,
        Self::Radius,
        Self::Diameter,
        Self::Angle,
        Self::ArcLength,
        Self::Area,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Linear => "Linear Dimension",
            Self::Radius => "Radius Dimension",
            Self::Diameter => "Diameter Dimension",
            Self::Angle => "Angle Dimension",
            Self::ArcLength => "Arc Length Dimension",
            Self::Area => "Area Dimension",
        }
    }

    fn default_value(self) -> f64 {
        match self {
            Self::Linear => 100.0,
            Self::Radius => 25.0,
            Self::Diameter => 50.0,
            Self::Angle => 90.0,
            Self::ArcLength => 52.36,
            Self::Area => 1200.0,
        }
    }

    fn default_label(self) -> String {
        match self {
            Self::Linear => format!("{:.2}", self.default_value()),
            Self::Radius => format!("R{:.2}", self.default_value()),
            Self::Diameter => format!("⌀{:.2}", self.default_value()),
            Self::Angle => format!("{:.1}°", self.default_value()),
            Self::ArcLength => "Arc".into(),
            Self::Area => "Area:".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TechDrawDimensionSetupState {
    pub kind: TechDrawDimensionKind,
    pub label: String,
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub offset: f64,
    pub center_x: f64,
    pub center_y: f64,
    pub radius: f64,
    pub value: f64,
    pub start_angle: f64,
    pub end_angle: f64,
    pub area_width: f64,
    pub area_height: f64,
}

impl TechDrawDimensionSetupState {
    pub fn new(kind: TechDrawDimensionKind, sheet_width: f64, sheet_height: f64) -> Self {
        let value = kind.default_value();
        let center_x = sheet_width * 0.50;
        let center_y = sheet_height * 0.55;
        let area_width = sheet_width.min(sheet_height) * 0.18;
        let area_height = sheet_width.min(sheet_height) * 0.10;
        Self {
            kind,
            label: kind.default_label(),
            x1: sheet_width * 0.25,
            y1: sheet_height * 0.82,
            x2: sheet_width * 0.75,
            y2: sheet_height * 0.82,
            offset: -12.0,
            center_x,
            center_y,
            radius: sheet_width.min(sheet_height) * 0.08,
            value,
            start_angle: 20.0,
            end_angle: 140.0,
            area_width,
            area_height,
        }
    }

    pub fn apply_kind(&mut self, kind: TechDrawDimensionKind, sheet_width: f64, sheet_height: f64) {
        *self = Self::new(kind, sheet_width, sheet_height);
    }

    pub fn apply_to_sheet(&self, sheet: &mut cadkernel_io::DrawingSheet) {
        let center = cadkernel_math::Point2::new(self.center_x, self.center_y);
        match self.kind {
            TechDrawDimensionKind::Linear => {
                sheet.dimensions.push(cadkernel_io::Dimension::Linear {
                    x1: self.x1,
                    y1: self.y1,
                    x2: self.x2,
                    y2: self.y2,
                    offset: self.offset,
                    text: self.label.clone(),
                })
            }
            TechDrawDimensionKind::Radius => {
                sheet.dimensions.push(cadkernel_io::Dimension::Radius {
                    cx: self.center_x,
                    cy: self.center_y,
                    r: self.radius.max(1e-6),
                    angle_deg: self.value,
                    text: self.label.clone(),
                })
            }
            TechDrawDimensionKind::Diameter => {
                sheet
                    .extended_dimensions
                    .push(cadkernel_io::DimensionType::DiameterDimension {
                        center,
                        diameter: self.value.max(1e-6),
                    });
            }
            TechDrawDimensionKind::Angle => {
                let arm_len = self.radius.max(1.0);
                let angle = self.value;
                let rad = angle.to_radians();
                sheet
                    .extended_dimensions
                    .push(cadkernel_io::DimensionType::AngleDimension {
                        vertex: center,
                        arm1_end: cadkernel_math::Point2::new(center.x + arm_len, center.y),
                        arm2_end: cadkernel_math::Point2::new(
                            center.x + arm_len * rad.cos(),
                            center.y - arm_len * rad.sin(),
                        ),
                        angle,
                    });
            }
            TechDrawDimensionKind::ArcLength => {
                sheet
                    .arc_length_dimensions
                    .push(cadkernel_io::arc_length_dimension(
                        center,
                        self.radius.max(1e-6),
                        self.start_angle,
                        self.end_angle,
                    ));
            }
            TechDrawDimensionKind::Area => {
                let half_w = self.area_width.max(1e-6) * 0.5;
                let half_h = self.area_height.max(1e-6) * 0.5;
                sheet.area_annotations.push(cadkernel_io::AreaAnnotation {
                    boundary: vec![
                        cadkernel_math::Point2::new(center.x - half_w, center.y - half_h),
                        cadkernel_math::Point2::new(center.x + half_w, center.y - half_h),
                        cadkernel_math::Point2::new(center.x + half_w, center.y + half_h),
                        cadkernel_math::Point2::new(center.x - half_w, center.y + half_h),
                    ],
                    area: self.value.max(0.0),
                    label_position: center,
                });
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TechDrawAnnotationKind {
    Text,
    RichText,
    Balloon,
    Leader,
    Weld,
    SurfaceFinish,
}

impl TechDrawAnnotationKind {
    pub const ALL: [Self; 6] = [
        Self::Text,
        Self::RichText,
        Self::Balloon,
        Self::Leader,
        Self::Weld,
        Self::SurfaceFinish,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "Text Annotation",
            Self::RichText => "Rich Text Annotation",
            Self::Balloon => "Balloon Annotation",
            Self::Leader => "Leader Line",
            Self::Weld => "Weld Symbol",
            Self::SurfaceFinish => "Surface Finish",
        }
    }

    fn default_text(self) -> &'static str {
        match self {
            Self::Text => "NOTE: Deburr all edges",
            Self::RichText => "<b>Rich</b> text note",
            Self::Balloon => "1",
            Self::Leader => "Leader callout",
            Self::Weld => "Fillet weld",
            Self::SurfaceFinish => "Ra 3.2",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TechDrawAnnotationSetupState {
    pub kind: TechDrawAnnotationKind,
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub end_x: f64,
    pub end_y: f64,
    pub font_size: f64,
    pub radius: f64,
    pub number: u32,
    pub weld_size: f64,
    pub weld_length: f64,
    pub roughness: f64,
}

impl TechDrawAnnotationSetupState {
    pub fn new(kind: TechDrawAnnotationKind, sheet_width: f64, sheet_height: f64) -> Self {
        Self {
            kind,
            text: kind.default_text().into(),
            x: sheet_width * 0.12,
            y: sheet_height * 0.22,
            end_x: sheet_width * 0.68,
            end_y: sheet_height * 0.34,
            font_size: 6.0,
            radius: 7.0,
            number: 1,
            weld_size: 8.0,
            weld_length: 25.0,
            roughness: 3.2,
        }
    }

    pub fn apply_kind(
        &mut self,
        kind: TechDrawAnnotationKind,
        sheet_width: f64,
        sheet_height: f64,
    ) {
        *self = Self::new(kind, sheet_width, sheet_height);
    }

    pub fn apply_to_sheet(&self, sheet: &mut cadkernel_io::DrawingSheet) {
        let pos = cadkernel_math::Point2::new(self.x, self.y);
        let end = cadkernel_math::Point2::new(self.end_x, self.end_y);
        match self.kind {
            TechDrawAnnotationKind::Text => {
                sheet.text_annotations.push(cadkernel_io::TextAnnotation {
                    position: pos,
                    text: self.text.clone(),
                    font_size: self.font_size,
                });
            }
            TechDrawAnnotationKind::RichText => {
                sheet
                    .rich_text_annotations
                    .push(cadkernel_io::rich_text_annotation(
                        pos,
                        &self.text,
                        self.font_size,
                    ));
            }
            TechDrawAnnotationKind::Balloon => {
                sheet
                    .balloon_annotations
                    .push(cadkernel_io::balloon_annotation(
                        pos,
                        end,
                        self.number,
                        self.radius.max(1e-6),
                    ));
            }
            TechDrawAnnotationKind::Leader => {
                sheet.leader_lines.push(cadkernel_io::LeaderLine {
                    start: pos,
                    end,
                    text: self.text.clone(),
                });
            }
            TechDrawAnnotationKind::Weld => {
                sheet.weld_symbols.push(cadkernel_io::weld_symbol(
                    pos,
                    cadkernel_io::WeldType::Fillet,
                    self.weld_size.max(1e-6),
                    self.weld_length.max(0.0),
                    0.0,
                    cadkernel_io::WeldContour::None,
                    cadkernel_io::WeldFinish::None,
                ));
            }
            TechDrawAnnotationKind::SurfaceFinish => {
                sheet
                    .surface_finish_symbols
                    .push(cadkernel_io::SurfaceFinishSymbol {
                        position: pos,
                        roughness: self.roughness.max(0.0),
                    });
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TechDrawCenterlineKind {
    Face,
    BetweenLines,
    CenterMark,
    BoltCircle,
}

impl TechDrawCenterlineKind {
    pub const ALL: [Self; 4] = [
        Self::Face,
        Self::BetweenLines,
        Self::CenterMark,
        Self::BoltCircle,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Face => "Face Centerline",
            Self::BetweenLines => "Centerline Between Lines",
            Self::CenterMark => "Center Mark",
            Self::BoltCircle => "Bolt Circle Centerlines",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TechDrawCenterlineSetupState {
    pub kind: TechDrawCenterlineKind,
    pub center_x: f64,
    pub center_y: f64,
    pub width: f64,
    pub height: f64,
    pub span: f64,
    pub gap: f64,
    pub extension: f64,
    pub mark_size: f64,
    pub radius: f64,
    pub bolt_count: u32,
    pub horizontal: bool,
}

impl TechDrawCenterlineSetupState {
    pub fn new(kind: TechDrawCenterlineKind, sheet_width: f64, sheet_height: f64) -> Self {
        let base = sheet_width.min(sheet_height);
        Self {
            kind,
            center_x: sheet_width * 0.50,
            center_y: sheet_height * 0.50,
            width: base * 0.32,
            height: base * 0.18,
            span: base * 0.22,
            gap: base * 0.04,
            extension: 5.0,
            mark_size: base * 0.05,
            radius: base * 0.09,
            bolt_count: 6,
            horizontal: true,
        }
    }

    pub fn apply_kind(
        &mut self,
        kind: TechDrawCenterlineKind,
        sheet_width: f64,
        sheet_height: f64,
    ) {
        *self = Self::new(kind, sheet_width, sheet_height);
    }

    pub fn apply_to_sheet(&self, sheet: &mut cadkernel_io::DrawingSheet) {
        let center = cadkernel_math::Point2::new(self.center_x, self.center_y);
        match self.kind {
            TechDrawCenterlineKind::Face => {
                let half_w = self.width.max(1e-6) * 0.5;
                let half_h = self.height.max(1e-6) * 0.5;
                let boundary = vec![
                    cadkernel_math::Point2::new(center.x - half_w, center.y - half_h),
                    cadkernel_math::Point2::new(center.x + half_w, center.y - half_h),
                    cadkernel_math::Point2::new(center.x + half_w, center.y + half_h),
                    cadkernel_math::Point2::new(center.x - half_w, center.y + half_h),
                ];
                sheet.centerlines.push(cadkernel_io::centerline_on_face(
                    &boundary,
                    self.extension.max(0.0),
                ));
            }
            TechDrawCenterlineKind::BetweenLines => {
                let span = self.span.max(1e-6);
                let gap = self.gap.max(0.0);
                let (l1_start, l1_end, l2_start, l2_end) = if self.horizontal {
                    (
                        cadkernel_math::Point2::new(center.x - span, center.y - gap),
                        cadkernel_math::Point2::new(center.x + span, center.y - gap),
                        cadkernel_math::Point2::new(center.x - span, center.y + gap),
                        cadkernel_math::Point2::new(center.x + span, center.y + gap),
                    )
                } else {
                    (
                        cadkernel_math::Point2::new(center.x - gap, center.y - span),
                        cadkernel_math::Point2::new(center.x - gap, center.y + span),
                        cadkernel_math::Point2::new(center.x + gap, center.y - span),
                        cadkernel_math::Point2::new(center.x + gap, center.y + span),
                    )
                };
                sheet
                    .centerlines
                    .push(cadkernel_io::centerline_between_lines(
                        l1_start,
                        l1_end,
                        l2_start,
                        l2_end,
                        self.extension.max(0.0),
                    ));
            }
            TechDrawCenterlineKind::CenterMark => {
                sheet.center_marks.push(cadkernel_io::CenterMark {
                    center,
                    size: self.mark_size.max(1e-6),
                });
            }
            TechDrawCenterlineKind::BoltCircle => {
                sheet
                    .bolt_circle_centerlines
                    .push(cadkernel_io::bolt_circle_centerlines(
                        center,
                        self.radius.max(1e-6),
                        self.bolt_count.max(1) as usize,
                        self.mark_size.max(1e-6),
                    ));
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TechDrawTemplatePreset {
    A4Landscape,
    A4Portrait,
    A3Landscape,
    Custom,
}

impl TechDrawTemplatePreset {
    pub const ALL: [Self; 4] = [
        Self::A4Landscape,
        Self::A4Portrait,
        Self::A3Landscape,
        Self::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::A4Landscape => "A4 Landscape",
            Self::A4Portrait => "A4 Portrait",
            Self::A3Landscape => "A3 Landscape",
            Self::Custom => "Custom",
        }
    }

    pub fn default_title(self) -> &'static str {
        match self {
            Self::A4Landscape => "A4 Landscape (Title Block)",
            Self::A4Portrait => "A4 Portrait",
            Self::A3Landscape => "A3 Landscape (Title Block)",
            Self::Custom => "Custom Drawing",
        }
    }

    pub fn dimensions(self) -> Option<(f64, f64)> {
        match self {
            Self::A4Landscape => Some((297.0, 210.0)),
            Self::A4Portrait => Some((210.0, 297.0)),
            Self::A3Landscape => Some((420.0, 297.0)),
            Self::Custom => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TechDrawPageSetupState {
    pub preset: TechDrawTemplatePreset,
    pub title: String,
    pub width: f64,
    pub height: f64,
}

impl TechDrawPageSetupState {
    pub fn new() -> Self {
        Self::from_preset(TechDrawTemplatePreset::A4Landscape)
    }

    pub fn from_sheet(sheet: &cadkernel_io::DrawingSheet) -> Self {
        let preset = match (sheet.width, sheet.height) {
            (w, h) if near(w, 297.0) && near(h, 210.0) => TechDrawTemplatePreset::A4Landscape,
            (w, h) if near(w, 210.0) && near(h, 297.0) => TechDrawTemplatePreset::A4Portrait,
            (w, h) if near(w, 420.0) && near(h, 297.0) => TechDrawTemplatePreset::A3Landscape,
            _ => TechDrawTemplatePreset::Custom,
        };
        Self {
            preset,
            title: sheet.title.clone(),
            width: sheet.width,
            height: sheet.height,
        }
    }

    pub fn from_preset(preset: TechDrawTemplatePreset) -> Self {
        let (width, height) = preset.dimensions().unwrap_or((297.0, 210.0));
        Self {
            preset,
            title: preset.default_title().into(),
            width,
            height,
        }
    }

    pub fn apply_preset(&mut self, preset: TechDrawTemplatePreset) {
        self.preset = preset;
        if let Some((width, height)) = preset.dimensions() {
            self.width = width;
            self.height = height;
        }
        self.title = preset.default_title().into();
    }

    pub fn to_sheet(&self) -> cadkernel_io::DrawingSheet {
        let mut sheet = cadkernel_io::DrawingSheet::a4_landscape();
        sheet.width = self.width;
        sheet.height = self.height;
        sheet.title = self.title.clone();
        sheet
    }
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_setup_default_is_a4_landscape() {
        let s = TechDrawPageSetupState::new();
        assert_eq!(s.preset, TechDrawTemplatePreset::A4Landscape);
        assert!((s.width - 297.0).abs() < 1e-6);
        assert!((s.height - 210.0).abs() < 1e-6);
    }

    #[test]
    fn page_setup_a3_preset_updates_title_and_size() {
        let mut s = TechDrawPageSetupState::new();
        s.apply_preset(TechDrawTemplatePreset::A3Landscape);
        assert_eq!(s.title, "A3 Landscape (Title Block)");
        assert!((s.width - 420.0).abs() < 1e-6);
        assert!((s.height - 297.0).abs() < 1e-6);
    }

    #[test]
    fn dimension_setup_linear_defaults_to_sheet_placement() {
        let s = TechDrawDimensionSetupState::new(TechDrawDimensionKind::Linear, 297.0, 210.0);
        assert_eq!(s.kind, TechDrawDimensionKind::Linear);
        assert_eq!(s.label, "100.00");
        assert!((s.x1 - 74.25).abs() < 1e-6);
        assert!((s.x2 - 222.75).abs() < 1e-6);
    }

    #[test]
    fn dimension_setup_diameter_pushes_extended_dimension() {
        let mut sheet = cadkernel_io::DrawingSheet::a4_landscape();
        let mut s = TechDrawDimensionSetupState::new(TechDrawDimensionKind::Diameter, 297.0, 210.0);
        s.value = 42.0;
        s.apply_to_sheet(&mut sheet);
        assert_eq!(sheet.extended_dimensions.len(), 1);
        match &sheet.extended_dimensions[0] {
            cadkernel_io::DimensionType::DiameterDimension { diameter, .. } => {
                assert!((*diameter - 42.0).abs() < 1e-6);
            }
            other => panic!("expected diameter dimension, got {other:?}"),
        }
    }

    #[test]
    fn annotation_setup_text_defaults_to_note() {
        let s = TechDrawAnnotationSetupState::new(TechDrawAnnotationKind::Text, 297.0, 210.0);
        assert_eq!(s.kind, TechDrawAnnotationKind::Text);
        assert_eq!(s.text, "NOTE: Deburr all edges");
        assert!((s.font_size - 6.0).abs() < 1e-6);
    }

    #[test]
    fn annotation_setup_balloon_pushes_balloon_annotation() {
        let mut sheet = cadkernel_io::DrawingSheet::a4_landscape();
        let mut s =
            TechDrawAnnotationSetupState::new(TechDrawAnnotationKind::Balloon, 297.0, 210.0);
        s.number = 7;
        s.apply_to_sheet(&mut sheet);
        assert_eq!(sheet.balloon_annotations.len(), 1);
        assert_eq!(sheet.balloon_annotations[0].text, "7");
    }

    #[test]
    fn centerline_setup_center_mark_defaults_to_sheet_center() {
        let s = TechDrawCenterlineSetupState::new(TechDrawCenterlineKind::CenterMark, 297.0, 210.0);
        assert_eq!(s.kind, TechDrawCenterlineKind::CenterMark);
        assert!((s.center_x - 148.5).abs() < 1e-6);
        assert!((s.center_y - 105.0).abs() < 1e-6);
        assert!((s.mark_size - 10.5).abs() < 1e-6);
    }

    #[test]
    fn centerline_setup_bolt_circle_pushes_bolt_circle_storage() {
        let mut sheet = cadkernel_io::DrawingSheet::a4_landscape();
        let mut s =
            TechDrawCenterlineSetupState::new(TechDrawCenterlineKind::BoltCircle, 297.0, 210.0);
        s.bolt_count = 8;
        s.radius = 32.0;
        s.apply_to_sheet(&mut sheet);
        assert_eq!(sheet.bolt_circle_centerlines.len(), 1);
        assert_eq!(sheet.bolt_circle_centerlines[0].bolt_count, 8);
        assert!((sheet.bolt_circle_centerlines[0].radius - 32.0).abs() < 1e-6);
    }

    #[test]
    fn view_setup_front_defaults_to_sheet_center_with_auto_scale() {
        let s = TechDrawViewSetupState::new(TechDrawViewKind::Front, 297.0, 210.0);
        assert_eq!(s.kind, TechDrawViewKind::Front);
        assert!(s.auto_scale);
        assert!((s.sheet_x - 148.5).abs() < 1e-6);
        assert!((s.sheet_y - 115.5).abs() < 1e-6);
        assert_eq!(s.scale_override(), None);
    }

    #[test]
    fn view_setup_applies_manual_sheet_placement() {
        let mut view = cadkernel_io::DrawingView {
            direction: cadkernel_io::ProjectionDir::Front,
            edges: Vec::new(),
            center_x: 0.0,
            center_y: 0.0,
            scale: 1.0,
            sheet_x: None,
            sheet_y: None,
            sheet_scale: None,
        };
        let mut s = TechDrawViewSetupState::new(TechDrawViewKind::Front, 297.0, 210.0);
        s.sheet_x = 120.0;
        s.sheet_y = 90.0;
        s.auto_scale = false;
        s.sheet_scale = 35.0;
        s.apply_placement(&mut view);
        assert_eq!(view.sheet_x, Some(120.0));
        assert_eq!(view.sheet_y, Some(90.0));
        assert_eq!(view.sheet_scale, Some(35.0));
    }
}
