//! Compound shape grouping without boolean operations.

use cadkernel_topology::{Handle, SolidData};

/// A compound groups multiple solids without performing boolean operations.
#[derive(Debug)]
pub struct Compound {
    pub solids: Vec<Handle<SolidData>>,
    pub name: String,
}

impl Compound {
    /// Creates a new empty compound with the given name.
    pub fn new(name: &str) -> Self {
        Self {
            solids: Vec::new(),
            name: name.to_string(),
        }
    }

    /// Adds a solid to this compound.
    pub fn add(&mut self, solid: Handle<SolidData>) {
        self.solids.push(solid);
    }

    /// Returns all solids as a flat list (exploding the compound).
    pub fn explode(&self) -> Vec<Handle<SolidData>> {
        self.solids.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;
    use cadkernel_math::Point3;
    use cadkernel_topology::BRepModel;

    #[test]
    fn test_compound_add_explode() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();

        let mut compound = Compound::new("test_compound");
        compound.add(r1.solid);
        compound.add(r2.solid);

        assert_eq!(compound.solids.len(), 2);
        assert_eq!(compound.name, "test_compound");

        let exploded = compound.explode();
        assert_eq!(exploded.len(), 2);
        assert_eq!(exploded[0].index(), r1.solid.index());
        assert_eq!(exploded[1].index(), r2.solid.index());
    }

    #[test]
    fn test_compound_empty() {
        let compound = Compound::new("empty");
        assert!(compound.solids.is_empty());
        assert!(compound.explode().is_empty());
    }
}
