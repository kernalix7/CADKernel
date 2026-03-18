//! PartDesign Body container for feature tree management.

use cadkernel_topology::{Handle, SolidData};

/// A PartDesign body that maintains a feature tree and tip solid.
#[derive(Debug)]
pub struct Body {
    pub name: String,
    pub features: Vec<BodyFeature>,
    pub tip: Option<Handle<SolidData>>,
}

/// A single feature entry in a body's feature tree.
#[derive(Debug)]
pub struct BodyFeature {
    pub name: String,
    pub kind: FeatureKind,
    pub solid: Handle<SolidData>,
}

/// The kind of feature operation that produced a body feature.
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
}
