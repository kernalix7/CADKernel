//! Entity property system: colors, materials, and arbitrary metadata.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Color
// ---------------------------------------------------------------------------

/// An RGBA colour with components in the `[0.0, 1.0]` range.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GRAY: Self = Self {
        r: 0.5,
        g: 0.5,
        b: 0.5,
        a: 1.0,
    };

    /// Creates an opaque colour from RGB components.
    pub fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Creates a colour with explicit alpha.
    pub fn rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }
}

// ---------------------------------------------------------------------------
// Material
// ---------------------------------------------------------------------------

/// A physically-inspired material description.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    pub density: f64,
    pub color: Color,
    pub metallic: f64,
    pub roughness: f64,
}

impl Material {
    /// Creates a new material with default appearance values.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            density: 0.0,
            color: Color::GRAY,
            metallic: 0.0,
            roughness: 0.5,
        }
    }

    /// Steel preset (7850 kg/m^3, metallic).
    pub fn steel() -> Self {
        Self {
            name: "Steel".into(),
            density: 7850.0,
            color: Color::rgb(0.7, 0.7, 0.73),
            metallic: 1.0,
            roughness: 0.3,
        }
    }

    /// Aluminum preset (2700 kg/m^3).
    pub fn aluminum() -> Self {
        Self {
            name: "Aluminum".into(),
            density: 2700.0,
            color: Color::rgb(0.8, 0.8, 0.82),
            metallic: 1.0,
            roughness: 0.25,
        }
    }

    /// ABS plastic preset (1050 kg/m^3).
    pub fn plastic_abs() -> Self {
        Self {
            name: "ABS".into(),
            density: 1050.0,
            color: Color::rgb(0.9, 0.9, 0.85),
            metallic: 0.0,
            roughness: 0.6,
        }
    }

    /// Wood preset (600 kg/m^3).
    pub fn wood() -> Self {
        Self {
            name: "Wood".into(),
            density: 600.0,
            color: Color::rgb(0.6, 0.4, 0.2),
            metallic: 0.0,
            roughness: 0.7,
        }
    }

    /// Builder: set density.
    pub fn with_density(mut self, density: f64) -> Self {
        self.density = density;
        self
    }

    /// Builder: set colour.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Builder: set metallic factor.
    pub fn with_metallic(mut self, metallic: f64) -> Self {
        self.metallic = metallic;
        self
    }

    /// Builder: set roughness factor.
    pub fn with_roughness(mut self, roughness: f64) -> Self {
        self.roughness = roughness;
        self
    }
}

// ---------------------------------------------------------------------------
// PropertyValue
// ---------------------------------------------------------------------------

/// A dynamically-typed value that can be attached to an entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyValue {
    String(String),
    Float(f64),
    Int(i64),
    Bool(bool),
}

// ---------------------------------------------------------------------------
// PropertyStore
// ---------------------------------------------------------------------------

/// Per-entity property storage keyed by a `u32` entity index.
///
/// Stores optional [`Material`] and arbitrary key-value metadata per entity.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PropertyStore {
    materials: HashMap<u32, Material>,
    metadata: HashMap<u32, HashMap<String, PropertyValue>>,
}

impl PropertyStore {
    /// Creates an empty property store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Assigns a material to the entity at `index`.
    pub fn set_material(&mut self, index: u32, material: Material) {
        self.materials.insert(index, material);
    }

    /// Returns the material for the entity at `index`, if any.
    pub fn get_material(&self, index: u32) -> Option<&Material> {
        self.materials.get(&index)
    }

    /// Sets a metadata key-value pair on the entity at `index`.
    pub fn set_metadata(&mut self, index: u32, key: impl Into<String>, value: PropertyValue) {
        self.metadata
            .entry(index)
            .or_default()
            .insert(key.into(), value);
    }

    /// Returns a metadata value for the entity at `index` and `key`.
    pub fn get_metadata(&self, index: u32, key: &str) -> Option<&PropertyValue> {
        self.metadata.get(&index)?.get(key)
    }
}
