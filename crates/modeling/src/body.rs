//! PartDesign Body container for feature tree management.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_topology::{Handle, SolidData};

/// A PartDesign body that maintains a feature tree and tip solid.
///
/// Features are appended sequentially via [`add_feature`](Self::add_feature).
/// The `tip` always points to the result of the last applied feature. This
/// mirrors the FreeCAD PartDesign Body workflow.
#[derive(Debug)]
pub struct Body {
    pub name: String,
    pub features: Vec<BodyFeature>,
    pub tip: Option<Handle<SolidData>>,
}

/// A single feature entry in a [`Body`]'s feature tree.
///
/// Stores the feature name, the kind of operation, and the resulting solid.
#[derive(Debug)]
pub struct BodyFeature {
    pub name: String,
    pub kind: FeatureKind,
    pub solid: Handle<SolidData>,
}

/// The kind of feature operation that produced a [`BodyFeature`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureKind {
    Pad,
    Pocket,
    Revolve,
    Groove,
    Fillet,
    Chamfer,
    Mirror,
    Pattern,
}

impl Body {
    /// Creates a new empty body with the given name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            features: Vec::new(),
            tip: None,
        }
    }

    /// Adds a feature to the body, updating the tip to the new solid.
    pub fn add_feature(&mut self, name: &str, kind: FeatureKind, solid: Handle<SolidData>) {
        self.features.push(BodyFeature {
            name: name.to_string(),
            kind,
            solid,
        });
        self.tip = Some(solid);
    }

    /// Returns the tip solid (last feature result), if any.
    pub fn tip_solid(&self) -> Option<Handle<SolidData>> {
        self.tip
    }

    /// Returns the number of features in this body.
    pub fn feature_count(&self) -> usize {
        self.features.len()
    }

    /// Suppresses (skips) a feature at the given index during rebuild.
    ///
    /// Suppressed features are removed from the feature list. If the
    /// suppressed feature was the tip, the tip moves to the previous feature.
    pub fn suppress_feature(&mut self, index: usize) -> Option<BodyFeature> {
        if index >= self.features.len() {
            return None;
        }
        let removed = self.features.remove(index);
        if self.tip == Some(removed.solid) {
            self.tip = self.features.last().map(|f| f.solid);
        }
        Some(removed)
    }

    /// Sets the rebuild tip to the feature at the given index.
    ///
    /// Features after the tip index are kept but ignored when querying
    /// the current solid state. The tip solid is updated to the indexed feature.
    pub fn set_tip(&mut self, index: usize) -> bool {
        if index >= self.features.len() {
            return false;
        }
        self.tip = Some(self.features[index].solid);
        true
    }

    /// Transfers a feature from another body into this body.
    ///
    /// Removes the feature at `feature_index` from `source_body` and appends
    /// it to the end of this body's feature list, updating tips in both bodies.
    pub fn move_object_to_body(
        &mut self,
        source_body: &mut Body,
        feature_index: usize,
    ) -> KernelResult<()> {
        if feature_index >= source_body.features.len() {
            return Err(KernelError::InvalidArgument(format!(
                "feature_index {} out of range (source has {} features)",
                feature_index,
                source_body.features.len(),
            )));
        }
        let feature = source_body.features.remove(feature_index);
        if source_body.tip == Some(feature.solid) {
            source_body.tip = source_body.features.last().map(|f| f.solid);
        }
        self.tip = Some(feature.solid);
        self.features.push(feature);
        Ok(())
    }

    /// Reorders a feature from one position to another.
    ///
    /// Moves the feature at `from` to the `to` position, shifting other
    /// features accordingly. Updates the tip to the last feature.
    pub fn move_feature(&mut self, from: usize, to: usize) -> bool {
        if from >= self.features.len() || to >= self.features.len() {
            return false;
        }
        let feature = self.features.remove(from);
        self.features.insert(to, feature);
        self.tip = self.features.last().map(|f| f.solid);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;
    use cadkernel_math::Point3;
    use cadkernel_topology::BRepModel;

    #[test]
    fn test_body_new() {
        let body = Body::new("Body1");
        assert_eq!(body.name, "Body1");
        assert_eq!(body.feature_count(), 0);
        assert!(body.tip_solid().is_none());
    }

    #[test]
    fn test_body_add_features() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut body = Body::new("Body1");
        body.add_feature("Base Pad", FeatureKind::Pad, r1.solid);
        assert_eq!(body.feature_count(), 1);
        assert_eq!(body.tip_solid(), Some(r1.solid));

        body.add_feature("Pocket", FeatureKind::Pocket, r2.solid);
        assert_eq!(body.feature_count(), 2);
        assert_eq!(body.tip_solid(), Some(r2.solid));
    }

    #[test]
    fn test_body_suppress_feature() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let r3 = make_box(&mut model, Point3::ORIGIN, 3.0, 3.0, 3.0).unwrap();

        let mut body = Body::new("Body1");
        body.add_feature("Pad1", FeatureKind::Pad, r1.solid);
        body.add_feature("Pad2", FeatureKind::Pad, r2.solid);
        body.add_feature("Pad3", FeatureKind::Pad, r3.solid);

        assert_eq!(body.feature_count(), 3);
        assert_eq!(body.tip_solid(), Some(r3.solid));

        // Suppress middle feature
        let removed = body.suppress_feature(1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "Pad2");
        assert_eq!(body.feature_count(), 2);
        assert_eq!(body.tip_solid(), Some(r3.solid));

        // Suppress last feature (tip)
        let removed = body.suppress_feature(1);
        assert!(removed.is_some());
        assert_eq!(body.tip_solid(), Some(r1.solid));
    }

    #[test]
    fn test_body_set_tip() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut body = Body::new("Body1");
        body.add_feature("Pad1", FeatureKind::Pad, r1.solid);
        body.add_feature("Pad2", FeatureKind::Pad, r2.solid);

        assert!(body.set_tip(0));
        assert_eq!(body.tip_solid(), Some(r1.solid));

        assert!(body.set_tip(1));
        assert_eq!(body.tip_solid(), Some(r2.solid));

        assert!(!body.set_tip(5));
    }

    #[test]
    fn test_body_move_feature() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let r3 = make_box(&mut model, Point3::ORIGIN, 3.0, 3.0, 3.0).unwrap();

        let mut body = Body::new("Body1");
        body.add_feature("Pad1", FeatureKind::Pad, r1.solid);
        body.add_feature("Pad2", FeatureKind::Pad, r2.solid);
        body.add_feature("Pad3", FeatureKind::Pad, r3.solid);

        assert!(body.move_feature(0, 2));
        assert_eq!(body.features[0].name, "Pad2");
        assert_eq!(body.features[1].name, "Pad3");
        assert_eq!(body.features[2].name, "Pad1");
        assert_eq!(body.tip_solid(), Some(r1.solid));

        assert!(!body.move_feature(0, 5));
    }

    #[test]
    fn test_body_feature_kinds() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut body = Body::new("TestBody");
        body.add_feature("pad", FeatureKind::Pad, r.solid);
        body.add_feature("fillet", FeatureKind::Fillet, r.solid);
        body.add_feature("chamfer", FeatureKind::Chamfer, r.solid);

        assert_eq!(body.features[0].kind, FeatureKind::Pad);
        assert_eq!(body.features[1].kind, FeatureKind::Fillet);
        assert_eq!(body.features[2].kind, FeatureKind::Chamfer);
    }

    #[test]
    fn test_move_object_to_body() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut source = Body::new("Source");
        source.add_feature("Pad1", FeatureKind::Pad, r1.solid);
        source.add_feature("Pad2", FeatureKind::Pad, r2.solid);

        let mut target = Body::new("Target");

        target.move_object_to_body(&mut source, 0).unwrap();
        assert_eq!(source.feature_count(), 1);
        assert_eq!(target.feature_count(), 1);
        assert_eq!(target.features[0].name, "Pad1");
        assert_eq!(target.tip_solid(), Some(r1.solid));
        assert_eq!(source.tip_solid(), Some(r2.solid));
    }

    #[test]
    fn test_move_object_to_body_out_of_range() {
        let mut source = Body::new("Source");
        let mut target = Body::new("Target");
        assert!(target.move_object_to_body(&mut source, 0).is_err());
    }

    #[test]
    fn test_body_suppress_first_feature() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut body = Body::new("Body1");
        body.add_feature("Pad1", FeatureKind::Pad, r1.solid);
        body.add_feature("Pad2", FeatureKind::Pad, r2.solid);

        let removed = body.suppress_feature(0);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "Pad1");
        assert_eq!(body.feature_count(), 1);
        assert_eq!(body.tip_solid(), Some(r2.solid));
    }

    #[test]
    fn test_body_suppress_out_of_range() {
        let mut body = Body::new("Body1");
        let removed = body.suppress_feature(0);
        assert!(removed.is_none());
    }

    #[test]
    fn test_body_set_tip_first() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut body = Body::new("Body1");
        body.add_feature("Pad1", FeatureKind::Pad, r1.solid);
        body.add_feature("Pad2", FeatureKind::Pad, r2.solid);

        assert!(body.set_tip(0));
        assert_eq!(body.tip_solid(), Some(r1.solid));
    }

    #[test]
    fn test_body_feature_revolve_groove() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut body = Body::new("Body1");
        body.add_feature("rev", FeatureKind::Revolve, r.solid);
        body.add_feature("groove", FeatureKind::Groove, r.solid);

        assert_eq!(body.features[0].kind, FeatureKind::Revolve);
        assert_eq!(body.features[1].kind, FeatureKind::Groove);
    }

    #[test]
    fn test_body_move_feature_invalid_from() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mut body = Body::new("Body1");
        body.add_feature("Pad1", FeatureKind::Pad, r.solid);
        assert!(!body.move_feature(5, 0));
    }

    #[test]
    fn test_body_move_feature_invalid_to() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mut body = Body::new("Body1");
        body.add_feature("Pad1", FeatureKind::Pad, r.solid);
        assert!(!body.move_feature(0, 5));
    }

    #[test]
    fn test_body_multiple_kinds() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut body = Body::new("Body");
        for &kind in &[
            FeatureKind::Pad, FeatureKind::Pocket, FeatureKind::Revolve,
            FeatureKind::Groove, FeatureKind::Fillet, FeatureKind::Chamfer,
            FeatureKind::Mirror, FeatureKind::Pattern,
        ] {
            body.add_feature("f", kind, r.solid);
        }
        assert_eq!(body.feature_count(), 8);
    }
}
