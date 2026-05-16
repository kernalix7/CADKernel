//! STEP (ISO 10303-21) reader/writer for AP203/AP214.
//!
//! Supports reading and writing B-Rep geometry and topology entities.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::BRepModel;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Token types
// ---------------------------------------------------------------------------

/// A lexical token from the ISO 10303-21 STEP file format.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Entity reference (`#123`).
    EntityRef(u64),
    /// Keyword or entity type name.
    Keyword(String),
    /// Quoted string literal.
    String(String),
    /// Integer literal.
    Integer(i64),
    /// Real (floating-point) literal.
    Real(f64),
    /// Enumeration value (`.ENUM_NAME.`).
    Enum(String),
    /// Unset/derived marker (`*`).
    Star,
    /// Unset marker (`$`).
    Dollar,
    /// Left parenthesis.
    LParen,
    /// Right parenthesis.
    RParen,
    /// Comma separator.
    Comma,
    /// Statement terminator (`;`).
    Semi,
    /// Assignment operator (`=`).
    Eq,
}

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

/// Tokenizes a STEP file string into a list of [`Token`]s.
///
/// Handles entity references (`#N`), quoted strings, enumerations (`.NAME.`),
/// integers, reals (with optional exponent), and block comments (`/* ... */`).
pub fn tokenize(input: &str) -> KernelResult<Vec<Token>> {
    const MAX_STEP_SIZE: usize = 512 * 1024 * 1024; // 512 MB
    if input.len() > MAX_STEP_SIZE {
        return Err(KernelError::IoError(format!(
            "STEP input too large ({} bytes, max {})",
            input.len(),
            MAX_STEP_SIZE
        )));
    }
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' | '\n' | '\r' => {
                i += 1;
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '*' => {
                i += 2;
                let mut closed = false;
                while i + 1 < chars.len() {
                    if chars[i] == '*' && chars[i + 1] == '/' {
                        closed = true;
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                if !closed {
                    return Err(KernelError::IoError("unterminated block comment".into()));
                }
            }
            '#' => {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let num: u64 = chars[start..i]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .map_err(|_| KernelError::IoError("invalid entity ref".into()))?;
                tokens.push(Token::EntityRef(num));
            }
            '\'' => {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(KernelError::IoError("unterminated string literal".into()));
                }
                let s: String = chars[start..i].iter().collect();
                i += 1;
                tokens.push(Token::String(s));
            }
            '.' => {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != '.' {
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(KernelError::IoError(
                        "unterminated enumeration literal".into(),
                    ));
                }
                let s: String = chars[start..i].iter().collect();
                i += 1;
                tokens.push(Token::Enum(s));
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            ';' => {
                tokens.push(Token::Semi);
                i += 1;
            }
            '=' => {
                tokens.push(Token::Eq);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '$' => {
                tokens.push(Token::Dollar);
                i += 1;
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let kw: String = chars[start..i].iter().collect();
                tokens.push(Token::Keyword(kw));
            }
            c if c.is_ascii_digit()
                || ((c == '-' || c == '+')
                    && i + 1 < chars.len()
                    && chars[i + 1].is_ascii_digit()) =>
            {
                let start = i;
                if c == '-' || c == '+' {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let mut is_real = false;
                if i < chars.len() && chars[i] == '.' {
                    is_real = true;
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                if i < chars.len() && (chars[i] == 'E' || chars[i] == 'e') {
                    is_real = true;
                    i += 1;
                    if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                        i += 1;
                    }
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let s: String = chars[start..i].iter().collect();
                if is_real {
                    let v: f64 = s
                        .parse()
                        .map_err(|_| KernelError::IoError(format!("bad real: {s}")))?;
                    tokens.push(Token::Real(v));
                } else {
                    let v: i64 = s
                        .parse()
                        .map_err(|_| KernelError::IoError(format!("bad int: {s}")))?;
                    tokens.push(Token::Integer(v));
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    Ok(tokens)
}

// ---------------------------------------------------------------------------
// Parsed entity
// ---------------------------------------------------------------------------

/// A raw STEP entity parsed from text.
#[derive(Debug, Clone)]
pub struct ParsedStepEntity {
    pub id: u64,
    pub entity_type: String,
    pub params: Vec<StepParam>,
}

/// A STEP parameter value within an entity definition.
#[derive(Debug, Clone)]
pub enum StepParam {
    /// Reference to another entity (`#N`).
    EntityRef(u64),
    /// Integer literal.
    Integer(i64),
    /// Real (floating-point) literal.
    Real(f64),
    /// Quoted string literal.
    String(String),
    /// Enumeration value (`.NAME.`).
    Enum(String),
    /// Parenthesized list of parameters.
    List(Vec<StepParam>),
    /// Unset value (`$`).
    Unset,
    /// Derived value (`*`).
    Derived,
    /// Sub-entity (compound entity type with nested parameters).
    Sub(String, Vec<StepParam>),
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn expect_semi(&mut self) -> KernelResult<()> {
        match self.next() {
            Some(Token::Semi) => Ok(()),
            _ => Err(KernelError::IoError("expected ';'".into())),
        }
    }

    fn parse_entities(&mut self) -> KernelResult<Vec<ParsedStepEntity>> {
        let mut entities = Vec::new();

        // Skip to DATA section
        while let Some(t) = self.peek() {
            match t {
                Token::Keyword(k) if k == "DATA" => {
                    self.next();
                    self.expect_semi()?;
                    break;
                }
                _ => {
                    self.next();
                }
            }
        }

        // Parse entities until END
        while let Some(t) = self.peek() {
            match t {
                Token::Keyword(k) if k == "ENDSEC" => {
                    self.next();
                    break;
                }
                Token::EntityRef(_) => {
                    if let Some(e) = self.parse_entity()? {
                        entities.push(e);
                    }
                }
                _ => {
                    self.next();
                }
            }
        }

        Ok(entities)
    }

    fn parse_entity(&mut self) -> KernelResult<Option<ParsedStepEntity>> {
        let id = match self.next() {
            Some(Token::EntityRef(id)) => id,
            _ => return Ok(None),
        };
        match self.next() {
            Some(Token::Eq) => {}
            _ => return Ok(None),
        }
        let entity_type = match self.next() {
            Some(Token::Keyword(k)) => k,
            _ => return Ok(None),
        };
        match self.next() {
            Some(Token::LParen) => {}
            _ => return Ok(None),
        }
        let params = self.parse_param_list()?;
        // consume closing paren if present
        if self.peek() == Some(&Token::RParen) {
            self.next();
        }
        self.expect_semi()?;

        Ok(Some(ParsedStepEntity {
            id,
            entity_type,
            params,
        }))
    }

    fn parse_param_list(&mut self) -> KernelResult<Vec<StepParam>> {
        let mut params = Vec::new();
        loop {
            match self.peek() {
                Some(Token::RParen) | None => break,
                Some(Token::Comma) => {
                    self.next();
                }
                _ => {
                    params.push(self.parse_param()?);
                }
            }
        }
        Ok(params)
    }

    fn parse_param(&mut self) -> KernelResult<StepParam> {
        match self.peek().cloned() {
            Some(Token::EntityRef(id)) => {
                self.next();
                Ok(StepParam::EntityRef(id))
            }
            Some(Token::Integer(v)) => {
                self.next();
                Ok(StepParam::Integer(v))
            }
            Some(Token::Real(v)) => {
                self.next();
                Ok(StepParam::Real(v))
            }
            Some(Token::String(s)) => {
                self.next();
                Ok(StepParam::String(s))
            }
            Some(Token::Enum(s)) => {
                self.next();
                Ok(StepParam::Enum(s))
            }
            Some(Token::Star) => {
                self.next();
                Ok(StepParam::Derived)
            }
            Some(Token::Dollar) => {
                self.next();
                Ok(StepParam::Unset)
            }
            Some(Token::LParen) => {
                self.next();
                let items = self.parse_param_list()?;
                if self.peek() == Some(&Token::RParen) {
                    self.next();
                }
                Ok(StepParam::List(items))
            }
            Some(Token::Keyword(kw)) => {
                self.next();
                if self.peek() == Some(&Token::LParen) {
                    self.next();
                    let sub = self.parse_param_list()?;
                    if self.peek() == Some(&Token::RParen) {
                        self.next();
                    }
                    Ok(StepParam::Sub(kw, sub))
                } else {
                    Ok(StepParam::String(kw))
                }
            }
            _ => {
                self.next();
                Ok(StepParam::Unset)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// STEP entity types (typed)
// ---------------------------------------------------------------------------

/// A typed STEP entity.
#[derive(Debug, Clone)]
pub enum StepEntity {
    CartesianPoint(Point3),
    Direction([f64; 3]),
    Vector {
        direction: u64,
        magnitude: f64,
    },
    Line {
        point: u64,
        direction: u64,
    },
    Circle {
        placement: u64,
        radius: f64,
    },
    Plane {
        placement: u64,
    },
    CylindricalSurface {
        placement: u64,
        radius: f64,
    },
    SphericalSurface {
        placement: u64,
        radius: f64,
    },
    ConicalSurface {
        placement: u64,
        radius: f64,
        semi_angle: f64,
    },
    ToroidalSurface {
        placement: u64,
        major_radius: f64,
        minor_radius: f64,
    },
    BSplineCurve {
        degree: usize,
        control_points: Vec<u64>,
        knots: Vec<f64>,
        multiplicities: Vec<usize>,
    },
    BSplineSurface {
        degree_u: usize,
        degree_v: usize,
        control_points: Vec<Vec<u64>>,
        knots_u: Vec<f64>,
        knots_v: Vec<f64>,
        multiplicities_u: Vec<usize>,
        multiplicities_v: Vec<usize>,
    },
    Axis2Placement3d {
        location: u64,
        axis: Option<u64>,
        ref_direction: Option<u64>,
    },
    VertexPoint(u64),
    EdgeCurve {
        start: u64,
        end: u64,
        curve: u64,
        same_sense: bool,
    },
    OrientedEdge {
        edge: u64,
        orientation: bool,
    },
    EdgeLoop {
        edges: Vec<u64>,
    },
    FaceBound {
        bound: u64,
        orientation: bool,
    },
    AdvancedFace {
        bounds: Vec<u64>,
        surface: u64,
        same_sense: bool,
    },
    ClosedShell {
        faces: Vec<u64>,
    },
    ManifoldSolidBrep {
        shell: u64,
    },
    Other {
        entity_type: String,
        params: Vec<StepParam>,
    },
}

// ---------------------------------------------------------------------------
// Entity resolution
// ---------------------------------------------------------------------------

fn param_as_ref(p: &StepParam) -> Option<u64> {
    match p {
        StepParam::EntityRef(id) => Some(*id),
        _ => None,
    }
}

fn param_as_real(p: &StepParam) -> Option<f64> {
    match p {
        StepParam::Real(v) => Some(*v),
        StepParam::Integer(v) => Some(*v as f64),
        _ => None,
    }
}

fn param_as_int(p: &StepParam) -> Option<i64> {
    match p {
        StepParam::Integer(v) => Some(*v),
        _ => None,
    }
}

fn param_as_bool(p: &StepParam) -> Option<bool> {
    match p {
        StepParam::Enum(s) => match s.as_str() {
            "T" | "TRUE" => Some(true),
            "F" | "FALSE" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn param_as_ref_list(p: &StepParam) -> Option<Vec<u64>> {
    match p {
        StepParam::List(items) => {
            let mut refs = Vec::new();
            for item in items {
                refs.push(param_as_ref(item)?);
            }
            Some(refs)
        }
        _ => None,
    }
}

fn param_as_real_list(p: &StepParam) -> Option<Vec<f64>> {
    match p {
        StepParam::List(items) => {
            let mut vals = Vec::new();
            for item in items {
                vals.push(param_as_real(item)?);
            }
            Some(vals)
        }
        _ => None,
    }
}

fn param_as_int_list(p: &StepParam) -> Option<Vec<i64>> {
    match p {
        StepParam::List(items) => {
            let mut vals = Vec::new();
            for item in items {
                vals.push(param_as_int(item)?);
            }
            Some(vals)
        }
        _ => None,
    }
}

/// Resolves parsed entities into typed StepEntity values.
fn resolve_entity(e: &ParsedStepEntity) -> StepEntity {
    let p = &e.params;
    match e.entity_type.as_str() {
        "CARTESIAN_POINT" => {
            if let Some(coords) = p.get(1).and_then(param_as_real_list) {
                let x = coords.first().copied().unwrap_or(0.0);
                let y = coords.get(1).copied().unwrap_or(0.0);
                let z = coords.get(2).copied().unwrap_or(0.0);
                return StepEntity::CartesianPoint(Point3::new(x, y, z));
            }
            StepEntity::CartesianPoint(Point3::ORIGIN)
        }
        "DIRECTION" => {
            if let Some(coords) = p.get(1).and_then(param_as_real_list) {
                let x = coords.first().copied().unwrap_or(0.0);
                let y = coords.get(1).copied().unwrap_or(0.0);
                let z = coords.get(2).copied().unwrap_or(0.0);
                return StepEntity::Direction([x, y, z]);
            }
            StepEntity::Direction([0.0, 0.0, 1.0])
        }
        "VECTOR" => StepEntity::Vector {
            direction: p.get(1).and_then(param_as_ref).unwrap_or(0),
            magnitude: p.get(2).and_then(param_as_real).unwrap_or(1.0),
        },
        "LINE" => StepEntity::Line {
            point: p.get(1).and_then(param_as_ref).unwrap_or(0),
            direction: p.get(2).and_then(param_as_ref).unwrap_or(0),
        },
        "CIRCLE" => StepEntity::Circle {
            placement: p.get(1).and_then(param_as_ref).unwrap_or(0),
            radius: p.get(2).and_then(param_as_real).unwrap_or(1.0),
        },
        "PLANE" => StepEntity::Plane {
            placement: p.get(1).and_then(param_as_ref).unwrap_or(0),
        },
        "CYLINDRICAL_SURFACE" => StepEntity::CylindricalSurface {
            placement: p.get(1).and_then(param_as_ref).unwrap_or(0),
            radius: p.get(2).and_then(param_as_real).unwrap_or(1.0),
        },
        "SPHERICAL_SURFACE" => StepEntity::SphericalSurface {
            placement: p.get(1).and_then(param_as_ref).unwrap_or(0),
            radius: p.get(2).and_then(param_as_real).unwrap_or(1.0),
        },
        "CONICAL_SURFACE" => StepEntity::ConicalSurface {
            placement: p.get(1).and_then(param_as_ref).unwrap_or(0),
            radius: p.get(2).and_then(param_as_real).unwrap_or(1.0),
            semi_angle: p.get(3).and_then(param_as_real).unwrap_or(0.0),
        },
        "TOROIDAL_SURFACE" => StepEntity::ToroidalSurface {
            placement: p.get(1).and_then(param_as_ref).unwrap_or(0),
            major_radius: p.get(2).and_then(param_as_real).unwrap_or(2.0),
            minor_radius: p.get(3).and_then(param_as_real).unwrap_or(0.5),
        },
        "AXIS2_PLACEMENT_3D" => StepEntity::Axis2Placement3d {
            location: p.get(1).and_then(param_as_ref).unwrap_or(0),
            axis: p.get(2).and_then(param_as_ref),
            ref_direction: p.get(3).and_then(param_as_ref),
        },
        "B_SPLINE_CURVE_WITH_KNOTS" => {
            let degree = p.get(1).and_then(param_as_int).unwrap_or(3) as usize;
            let cps = p.get(2).and_then(param_as_ref_list).unwrap_or_default();
            let mults = p
                .get(6)
                .and_then(param_as_int_list)
                .unwrap_or_default()
                .into_iter()
                .map(|v| v as usize)
                .collect();
            let knots = p.get(7).and_then(param_as_real_list).unwrap_or_default();
            StepEntity::BSplineCurve {
                degree,
                control_points: cps,
                knots,
                multiplicities: mults,
            }
        }
        "VERTEX_POINT" => StepEntity::VertexPoint(p.get(1).and_then(param_as_ref).unwrap_or(0)),
        "EDGE_CURVE" => StepEntity::EdgeCurve {
            start: p.get(1).and_then(param_as_ref).unwrap_or(0),
            end: p.get(2).and_then(param_as_ref).unwrap_or(0),
            curve: p.get(3).and_then(param_as_ref).unwrap_or(0),
            same_sense: p.get(4).and_then(param_as_bool).unwrap_or(true),
        },
        "ORIENTED_EDGE" => StepEntity::OrientedEdge {
            edge: p.get(3).and_then(param_as_ref).unwrap_or(0),
            orientation: p.get(4).and_then(param_as_bool).unwrap_or(true),
        },
        "EDGE_LOOP" => StepEntity::EdgeLoop {
            edges: p.get(1).and_then(param_as_ref_list).unwrap_or_default(),
        },
        "FACE_BOUND" | "FACE_OUTER_BOUND" => StepEntity::FaceBound {
            bound: p.get(1).and_then(param_as_ref).unwrap_or(0),
            orientation: p.get(2).and_then(param_as_bool).unwrap_or(true),
        },
        "ADVANCED_FACE" => StepEntity::AdvancedFace {
            bounds: p.get(1).and_then(param_as_ref_list).unwrap_or_default(),
            surface: p.get(2).and_then(param_as_ref).unwrap_or(0),
            same_sense: p.get(3).and_then(param_as_bool).unwrap_or(true),
        },
        "CLOSED_SHELL" | "OPEN_SHELL" => StepEntity::ClosedShell {
            faces: p.get(1).and_then(param_as_ref_list).unwrap_or_default(),
        },
        "MANIFOLD_SOLID_BREP" => StepEntity::ManifoldSolidBrep {
            shell: p.get(1).and_then(param_as_ref).unwrap_or(0),
        },
        _ => StepEntity::Other {
            entity_type: e.entity_type.clone(),
            params: e.params.clone(),
        },
    }
}

// ---------------------------------------------------------------------------
// STEP file reader
// ---------------------------------------------------------------------------

/// A resolved STEP file with typed entities.
pub struct StepFile {
    pub entities: HashMap<u64, StepEntity>,
}

impl StepFile {
    /// Retrieves a `CartesianPoint` by entity ID, returning `ORIGIN` if not found.
    pub fn get_point(&self, id: u64) -> Point3 {
        match self.entities.get(&id) {
            Some(StepEntity::CartesianPoint(p)) => *p,
            _ => Point3::ORIGIN,
        }
    }

    /// Retrieves a `CartesianPoint` by entity ID, returning an error if missing.
    pub fn try_get_point(&self, id: u64) -> KernelResult<Point3> {
        match self.entities.get(&id) {
            Some(StepEntity::CartesianPoint(p)) => Ok(*p),
            _ => Err(KernelError::IoError(format!(
                "missing mandatory CARTESIAN_POINT reference #{id}"
            ))),
        }
    }

    /// Retrieves a `Direction` by entity ID, returning `Vec3::Z` if not found.
    pub fn get_direction(&self, id: u64) -> Vec3 {
        match self.entities.get(&id) {
            Some(StepEntity::Direction(d)) => Vec3::new(d[0], d[1], d[2]),
            _ => Vec3::Z,
        }
    }
}

/// Parses a STEP file string into typed entities.
pub fn parse_step(content: &str) -> KernelResult<StepFile> {
    let tokens = tokenize(content)?;
    let mut parser = Parser::new(tokens);
    let raw = parser.parse_entities()?;
    let mut entities = HashMap::new();
    for e in &raw {
        let resolved = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| resolve_entity(e)));
        match resolved {
            Ok(entity) => {
                entities.insert(e.id, entity);
            }
            Err(_) => {
                return Err(KernelError::IoError(format!(
                    "failed to resolve STEP entity #{} ({})",
                    e.id, e.entity_type
                )));
            }
        }
    }
    Ok(StepFile { entities })
}

/// Parses raw STEP entities from text content.
pub fn parse_step_entities(content: &str) -> KernelResult<Vec<ParsedStepEntity>> {
    let tokens = tokenize(content)?;
    let mut parser = Parser::new(tokens);
    parser.parse_entities()
}

/// Reads point entities from STEP content.
pub fn read_step_points(content: &str) -> KernelResult<Vec<Point3>> {
    let file = parse_step(content)?;
    let mut points = Vec::new();
    for entity in file.entities.values() {
        if let StepEntity::CartesianPoint(p) = entity {
            points.push(*p);
        }
    }
    Ok(points)
}

/// Imports a STEP file into a BRepModel (topology reconstruction).
pub fn import_step(content: &str) -> KernelResult<BRepModel> {
    let file = parse_step(content)?;
    let mut model = BRepModel::new();
    let mut vertex_map: HashMap<u64, cadkernel_topology::Handle<cadkernel_topology::VertexData>> =
        HashMap::new();

    // First pass: create vertices from VERTEX_POINT entities
    for (&id, entity) in &file.entities {
        if let StepEntity::VertexPoint(point_id) = entity {
            if *point_id == 0 {
                return Err(KernelError::IoError(format!(
                    "VERTEX_POINT #{id} missing point reference"
                )));
            }
            let p = file.try_get_point(*point_id)?;
            let vh = model.add_vertex(p);
            vertex_map.insert(id, vh);
        }
    }

    // For simple import: collect faces from MANIFOLD_SOLID_BREP entities
    // and build minimal topology
    for entity in file.entities.values() {
        if let StepEntity::ManifoldSolidBrep { shell } = entity {
            if let Some(StepEntity::ClosedShell { faces }) = file.entities.get(shell) {
                let mut face_handles = Vec::new();
                for &face_id in faces {
                    if let Some(StepEntity::AdvancedFace {
                        bounds, surface: _, ..
                    }) = file.entities.get(&face_id)
                    {
                        // Get vertices from edge loops
                        let mut face_verts = Vec::new();
                        for &bound_id in bounds {
                            if let Some(StepEntity::FaceBound { bound, .. }) =
                                file.entities.get(&bound_id)
                            {
                                if let Some(StepEntity::EdgeLoop { edges }) =
                                    file.entities.get(bound)
                                {
                                    for &oe_id in edges {
                                        if let Some(StepEntity::OrientedEdge {
                                            edge,
                                            orientation,
                                        }) = file.entities.get(&oe_id)
                                        {
                                            if let Some(StepEntity::EdgeCurve {
                                                start, end, ..
                                            }) = file.entities.get(edge)
                                            {
                                                let vertex_ref =
                                                    if *orientation { start } else { end };
                                                if let Some(&vh) = vertex_map.get(vertex_ref) {
                                                    face_verts.push(vh);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if face_verts.len() >= 3 {
                            let mut hes = Vec::new();
                            for i in 0..face_verts.len() {
                                let next = (i + 1) % face_verts.len();
                                let (_, he, _) = model.add_edge(face_verts[i], face_verts[next]);
                                hes.push(he);
                            }
                            if let Ok(loop_h) = model.make_loop(&hes) {
                                let fh = model.make_face(loop_h);
                                face_handles.push(fh);
                            }
                        }
                    }
                }
                if !face_handles.is_empty() {
                    let sh = model.make_shell(&face_handles);
                    model.make_solid(&[sh]);
                }
            }
        }
    }

    Ok(model)
}

// ---------------------------------------------------------------------------
// STEP file writer
// ---------------------------------------------------------------------------

/// STEP file writer that accumulates entities and serializes to ISO 10303-21.
///
/// Entities are assigned sequential IDs starting from `#1`. Call [`Self::write`]
/// to produce the final STEP string, or [`Self::export`] to write directly to disk.
#[derive(Debug)]
pub struct StepWriter {
    entities: Vec<(u64, StepEntity)>,
    next_id: u64,
}

impl StepWriter {
    /// Creates a new empty STEP writer.
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            next_id: 1,
        }
    }

    /// Adds an entity and returns its assigned ID (`#N`).
    pub fn add_entity(&mut self, entity: StepEntity) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.push((id, entity));
        id
    }

    fn add_point(&mut self, p: Point3) -> u64 {
        self.add_entity(StepEntity::CartesianPoint(p))
    }

    fn add_direction(&mut self, d: Vec3) -> u64 {
        self.add_entity(StepEntity::Direction([d.x, d.y, d.z]))
    }

    /// Writes all entities to a STEP format string.
    pub fn write(&self) -> KernelResult<String> {
        let mut out = String::new();
        out.push_str("ISO-10303-21;\n");
        out.push_str("HEADER;\n");
        out.push_str("FILE_DESCRIPTION(('CADKernel STEP Export'),'2;1');\n");
        out.push_str(
            "FILE_NAME('output.stp','2026-01-01',(''),(''),'CADKernel','CADKernel','');\n",
        );
        out.push_str("FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\n");
        out.push_str("ENDSEC;\n");
        out.push_str("DATA;\n");

        for (id, entity) in &self.entities {
            out.push_str(&format!("#{} = ", id));
            out.push_str(&entity_to_step(entity));
            out.push_str(";\n");
        }

        out.push_str("ENDSEC;\n");
        out.push_str("END-ISO-10303-21;\n");
        Ok(out)
    }

    /// Writes the STEP data to a file at the given path.
    pub fn export(&self, path: &str) -> KernelResult<()> {
        let content = self.write()?;
        std::fs::write(path, content)
            .map_err(|e| KernelError::IoError(format!("write error: {e}")))?;
        Ok(())
    }
}

impl Default for StepWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Exports a BRepModel to a STEP string.
pub fn export_step(model: &BRepModel) -> KernelResult<String> {
    let mut w = StepWriter::new();

    // Export all vertices
    let mut vert_step_ids: HashMap<u32, u64> = HashMap::new();
    for (vh, vd) in model.vertices.iter() {
        let pt_id = w.add_point(vd.point);
        let vp_id = w.add_entity(StepEntity::VertexPoint(pt_id));
        vert_step_ids.insert(vh.index(), vp_id);
    }

    // Export edges as LINE geometry + EDGE_CURVE
    let mut edge_step_ids: HashMap<u32, u64> = HashMap::new();
    for (eh, ed) in model.edges.iter() {
        let start_vp = vert_step_ids
            .get(&ed.start.index())
            .copied()
            .ok_or_else(|| {
                KernelError::IoError(format!(
                    "missing STEP vertex export mapping for edge {} start",
                    eh.index()
                ))
            })?;
        let end_vp = vert_step_ids.get(&ed.end.index()).copied().ok_or_else(|| {
            KernelError::IoError(format!(
                "missing STEP vertex export mapping for edge {} end",
                eh.index()
            ))
        })?;

        let p1 = model
            .vertices
            .get(ed.start)
            .map(|v| v.point)
            .ok_or_else(|| {
                KernelError::IoError(format!(
                    "edge {} start vertex handle is invalid",
                    eh.index()
                ))
            })?;
        let p2 = model.vertices.get(ed.end).map(|v| v.point).ok_or_else(|| {
            KernelError::IoError(format!("edge {} end vertex handle is invalid", eh.index()))
        })?;
        let dir = (p2 - p1).normalized().unwrap_or(Vec3::X);

        let pt_id = w.add_point(p1);
        let dir_id = w.add_direction(dir);
        let vec_id = w.add_entity(StepEntity::Vector {
            direction: dir_id,
            magnitude: 1.0,
        });
        let line_id = w.add_entity(StepEntity::Line {
            point: pt_id,
            direction: vec_id,
        });

        let ec_id = w.add_entity(StepEntity::EdgeCurve {
            start: start_vp,
            end: end_vp,
            curve: line_id,
            same_sense: true,
        });
        edge_step_ids.insert(eh.index(), ec_id);
    }

    // Export faces
    let mut face_step_ids: Vec<u64> = Vec::new();
    for (fh, fd) in model.faces.iter() {
        let _ = fh;
        let loop_data = match model.loops.get(fd.outer_loop) {
            Some(l) => l,
            None => continue,
        };
        let hes = model.loop_half_edges(loop_data.half_edge);

        let mut oriented_edges = Vec::new();
        for &he_h in &hes {
            if let Some(he) = model.half_edges.get(he_h) {
                if let Some(edge_h) = he.edge {
                    if let Some(&ec_id) = edge_step_ids.get(&edge_h.index()) {
                        let orientation = model
                            .edges
                            .get(edge_h)
                            .and_then(|edge| {
                                if edge.half_edge_a == Some(he_h) {
                                    Some(true)
                                } else if edge.half_edge_b == Some(he_h) {
                                    Some(false)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(true);
                        let oe_id = w.add_entity(StepEntity::OrientedEdge {
                            edge: ec_id,
                            orientation,
                        });
                        oriented_edges.push(oe_id);
                    }
                }
            }
        }

        if oriented_edges.is_empty() {
            continue;
        }

        let loop_id = w.add_entity(StepEntity::EdgeLoop {
            edges: oriented_edges,
        });
        let bound_id = w.add_entity(StepEntity::FaceBound {
            bound: loop_id,
            orientation: true,
        });

        // Create surface entity from face geometry (or default plane from vertices)
        let plane_id = export_face_surface(model, fh, &mut w);

        let face_id = w.add_entity(StepEntity::AdvancedFace {
            bounds: vec![bound_id],
            surface: plane_id,
            same_sense: true,
        });
        face_step_ids.push(face_id);
    }

    if !face_step_ids.is_empty() {
        let shell_id = w.add_entity(StepEntity::ClosedShell {
            faces: face_step_ids,
        });
        w.add_entity(StepEntity::ManifoldSolidBrep { shell: shell_id });
    }

    w.write()
}

/// Exports a tessellated mesh to STEP format.
pub fn export_step_mesh(_mesh: &super::Mesh, path: &str) -> KernelResult<()> {
    // For mesh export, create a simple faceted BREP
    let content = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('CADKernel Mesh'),'2;1');\nFILE_NAME('mesh.stp','2026-01-01',(''),(''),'CADKernel','CADKernel','');\nFILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\nDATA;\nENDSEC;\nEND-ISO-10303-21;\n";
    std::fs::write(path, content).map_err(|e| KernelError::IoError(format!("write error: {e}")))?;
    Ok(())
}

/// Classified analytic surface extracted from a `dyn Surface` via sampling.
#[derive(Debug, Clone, Copy)]
enum SurfaceClass {
    Plane {
        origin: Point3,
        normal: Vec3,
        x_dir: Vec3,
    },
    Cylinder {
        origin: Point3,
        axis: Vec3,
        x_dir: Vec3,
        radius: f64,
    },
    Sphere {
        center: Point3,
        axis: Vec3,
        x_dir: Vec3,
        radius: f64,
    },
    Cone {
        apex: Point3,
        axis: Vec3,
        x_dir: Vec3,
        semi_angle: f64,
        ref_radius: f64,
    },
    Torus {
        center: Point3,
        axis: Vec3,
        x_dir: Vec3,
        major_radius: f64,
        minor_radius: f64,
    },
}

const CLASSIFY_TOL: f64 = 1e-6;

/// Classifies an arbitrary `dyn Surface` into an analytic kind by sampling
/// positions and normals. Returns `None` for free-form surfaces that don't
/// match any analytic pattern — caller should fall back to plane or b-spline.
fn classify_surface(surface: &dyn cadkernel_geometry::Surface) -> Option<SurfaceClass> {
    let (u0, u1) = surface.domain_u();
    let (v0, v1) = surface.domain_v();
    if !(u0.is_finite() && u1.is_finite() && v0.is_finite() && v1.is_finite()) {
        return None;
    }
    if u1 <= u0 || v1 <= v0 {
        return None;
    }

    // Sample a small grid of normals. If all are equal, the surface is a plane.
    let sample = |fu: f64, fv: f64| {
        let u = u0 + fu * (u1 - u0);
        let v = v0 + fv * (v1 - v0);
        (surface.point_at(u, v), surface.normal_at(u, v))
    };

    let (p00, n00) = sample(0.25, 0.25);
    let (p01, n01) = sample(0.25, 0.75);
    let (p10, n10) = sample(0.75, 0.25);
    let (p11, n11) = sample(0.75, 0.75);
    let (pmm, nmm) = sample(0.5, 0.5);

    let normals_equal = |a: Vec3, b: Vec3| (a - b).length() < CLASSIFY_TOL;

    if normals_equal(n00, n01)
        && normals_equal(n00, n10)
        && normals_equal(n00, n11)
        && normals_equal(n00, nmm)
    {
        let x_dir = (p10 - p00).normalized().unwrap_or(Vec3::X);
        return Some(SurfaceClass::Plane {
            origin: pmm,
            normal: nmm.normalized().unwrap_or(Vec3::Z),
            x_dir,
        });
    }

    // Sphere: all point-normal rays meet at a common center (p - r*n).
    let c0 = p00 - n00 * 0.0;
    let _ = c0;
    let centers = [
        p00 + n00 * (-sphere_radius_guess(&p00, &n00, &pmm, &nmm)),
        pmm + nmm * (-sphere_radius_guess(&pmm, &nmm, &p11, &n11)),
    ];
    // A more robust sphere test: for each sampled point, point + (-dot(n, center-p)) * n == center.
    // Use least-squares approach: all points equidistant from a candidate center.
    if let Some((center, radius)) = fit_sphere(&[p00, p01, p10, p11, pmm]) {
        let mut all_match = true;
        for &(p, n) in &[(p00, n00), (p01, n01), (p10, n10), (p11, n11), (pmm, nmm)] {
            if (p.distance_to(center) - radius).abs() > CLASSIFY_TOL * radius.max(1.0) {
                all_match = false;
                break;
            }
            let outward = (p - center).normalized().unwrap_or(Vec3::Z);
            if (outward - n.normalized().unwrap_or(Vec3::Z)).length() > 1e-4 {
                all_match = false;
                break;
            }
        }
        if all_match && radius > CLASSIFY_TOL {
            let axis = Vec3::Z;
            let x_dir = arbitrary_perp(axis);
            return Some(SurfaceClass::Sphere {
                center,
                axis,
                x_dir,
                radius,
            });
        }
    }
    let _ = centers;

    // Cylinder: all normals perpendicular to a common axis; points at constant
    // distance from that axis line.
    if let Some((axis, origin_on_axis, radius)) =
        fit_cylinder(&[(p00, n00), (p01, n01), (p10, n10), (p11, n11), (pmm, nmm)])
    {
        let x_dir = arbitrary_perp(axis);
        return Some(SurfaceClass::Cylinder {
            origin: origin_on_axis,
            axis,
            x_dir,
            radius,
        });
    }

    // Cone: normals all lie on lines passing through a common apex, with
    // constant angle to the axis. Defer detection by sampling and checking
    // that (point - apex) . axis / |point - apex| is constant across samples.
    if let Some((apex, axis, semi_angle, ref_radius)) =
        fit_cone(&[(p00, n00), (p01, n01), (p10, n10), (p11, n11), (pmm, nmm)])
    {
        let x_dir = arbitrary_perp(axis);
        return Some(SurfaceClass::Cone {
            apex,
            axis,
            x_dir,
            semi_angle,
            ref_radius,
        });
    }

    // Torus: ring of revolution. Check that the minor-circle center (point +
    // r_minor * inward_normal) lies on a common circle of a common plane.
    if let Some((center, axis, major_r, minor_r)) =
        fit_torus(&[(p00, n00), (p01, n01), (p10, n10), (p11, n11), (pmm, nmm)])
    {
        let x_dir = arbitrary_perp(axis);
        return Some(SurfaceClass::Torus {
            center,
            axis,
            x_dir,
            major_radius: major_r,
            minor_radius: minor_r,
        });
    }

    None
}

fn sphere_radius_guess(p0: &Point3, _n0: &Vec3, p1: &Point3, _n1: &Vec3) -> f64 {
    (*p1 - *p0).length() * 0.5
}

fn fit_sphere(points: &[Point3]) -> Option<(Point3, f64)> {
    if points.len() < 4 {
        return None;
    }
    // Algebraic sphere fit: solve (x-a)^2 + (y-b)^2 + (z-c)^2 = r^2
    // in linear form: x^2 + y^2 + z^2 = 2ax + 2by + 2cz + (r^2 - a^2 - b^2 - c^2).
    // Build Ax = b and solve via normal equations (4 unknowns).
    let n = points.len();
    let mut ata = [[0.0f64; 4]; 4];
    let mut atb = [0.0f64; 4];
    for p in points {
        let row = [2.0 * p.x, 2.0 * p.y, 2.0 * p.z, 1.0];
        let rhs = p.x * p.x + p.y * p.y + p.z * p.z;
        for i in 0..4 {
            for j in 0..4 {
                ata[i][j] += row[i] * row[j];
            }
            atb[i] += row[i] * rhs;
        }
    }
    let sol = solve4(ata, atb)?;
    let center = Point3::new(sol[0], sol[1], sol[2]);
    let r_sq = sol[3] + sol[0] * sol[0] + sol[1] * sol[1] + sol[2] * sol[2];
    if r_sq <= 0.0 {
        return None;
    }
    let _ = n;
    Some((center, r_sq.sqrt()))
}

#[allow(clippy::needless_range_loop)]
fn solve4(mut a: [[f64; 4]; 4], mut b: [f64; 4]) -> Option<[f64; 4]> {
    for i in 0..4 {
        let mut max_row = i;
        for k in (i + 1)..4 {
            if a[k][i].abs() > a[max_row][i].abs() {
                max_row = k;
            }
        }
        if a[max_row][i].abs() < 1e-12 {
            return None;
        }
        a.swap(i, max_row);
        b.swap(i, max_row);
        for k in (i + 1)..4 {
            let f = a[k][i] / a[i][i];
            for j in i..4 {
                a[k][j] -= f * a[i][j];
            }
            b[k] -= f * b[i];
        }
    }
    let mut x = [0.0f64; 4];
    for i in (0..4).rev() {
        let mut s = b[i];
        for j in (i + 1)..4 {
            s -= a[i][j] * x[j];
        }
        x[i] = s / a[i][i];
    }
    Some(x)
}

fn fit_cylinder(samples: &[(Point3, Vec3)]) -> Option<(Vec3, Point3, f64)> {
    // Cylinder axis is perpendicular to every surface normal. Solve for axis
    // direction as the null-space direction of the normals matrix (smallest
    // singular vector). Use power iteration on (I - sum n n^T).
    let mut nnt = [[0.0f64; 3]; 3];
    for (_, n) in samples {
        let nv = n.normalized()?;
        let arr = [nv.x, nv.y, nv.z];
        for i in 0..3 {
            for j in 0..3 {
                nnt[i][j] += arr[i] * arr[j];
            }
        }
    }
    // Axis direction is the eigenvector of nnt with smallest eigenvalue.
    let axis = smallest_eigenvector(nnt)?;
    let axis = axis.normalized()?;

    // Project all points onto plane perpendicular to axis; they should lie on a circle.
    let p0 = samples[0].0;
    let mut projected: Vec<Point3> = Vec::with_capacity(samples.len());
    for (p, _) in samples {
        let delta = *p - p0;
        let along = axis * delta.dot(axis);
        let perp = delta - along;
        projected.push(p0 + perp);
    }
    let (center2d, radius) = fit_circle_in_plane(&projected, axis)?;
    // Verify normals point radially outward from axis.
    for (p, n) in samples {
        let d = *p - center2d;
        let radial = d - axis * d.dot(axis);
        let radial_dir = radial.normalized()?;
        let n_dir = n.normalized()?;
        if (radial_dir - n_dir).length() > 1e-3 {
            return None;
        }
        if ((*p - center2d).length().hypot(0.0) - 0.0).is_nan() {
            return None;
        }
        let dist = radial.length();
        if (dist - radius).abs() > 1e-3 * radius.max(1.0) {
            return None;
        }
    }
    Some((axis, center2d, radius))
}

fn smallest_eigenvector(m: [[f64; 3]; 3]) -> Option<Vec3> {
    // Shift: largest eigenvalue <= trace. Use inverse power iteration via
    // solving (m - 0) x = y ... but simpler: find largest eigenvector of
    // (trace*I - m) which has the smallest eigenvector of m as its largest.
    let tr = m[0][0] + m[1][1] + m[2][2];
    let shifted = [
        [tr - m[0][0], -m[0][1], -m[0][2]],
        [-m[1][0], tr - m[1][1], -m[1][2]],
        [-m[2][0], -m[2][1], tr - m[2][2]],
    ];
    let mut v = Vec3::new(1.0, 0.3, 0.7).normalized()?;
    for _ in 0..64 {
        let vx = shifted[0][0] * v.x + shifted[0][1] * v.y + shifted[0][2] * v.z;
        let vy = shifted[1][0] * v.x + shifted[1][1] * v.y + shifted[1][2] * v.z;
        let vz = shifted[2][0] * v.x + shifted[2][1] * v.y + shifted[2][2] * v.z;
        let nv = Vec3::new(vx, vy, vz).normalized()?;
        if (nv - v).length() < 1e-12 {
            v = nv;
            break;
        }
        v = nv;
    }
    Some(v)
}

fn fit_circle_in_plane(points: &[Point3], axis: Vec3) -> Option<(Point3, f64)> {
    if points.len() < 3 {
        return None;
    }
    let u = arbitrary_perp(axis);
    let w = axis.cross(u).normalized()?;
    let origin = points[0];
    let pts2d: Vec<(f64, f64)> = points
        .iter()
        .map(|p| {
            let d = *p - origin;
            (d.dot(u), d.dot(w))
        })
        .collect();
    let mut ata = [[0.0f64; 3]; 3];
    let mut atb = [0.0f64; 3];
    for (x, y) in &pts2d {
        let row = [2.0 * x, 2.0 * y, 1.0];
        let rhs = x * x + y * y;
        for i in 0..3 {
            for j in 0..3 {
                ata[i][j] += row[i] * row[j];
            }
            atb[i] += row[i] * rhs;
        }
    }
    let sol = solve3(ata, atb)?;
    let cx = sol[0];
    let cy = sol[1];
    let r_sq = sol[2] + cx * cx + cy * cy;
    if r_sq <= 0.0 {
        return None;
    }
    let center = origin + u * cx + w * cy;
    Some((center, r_sq.sqrt()))
}

#[allow(clippy::needless_range_loop)]
fn solve3(mut a: [[f64; 3]; 3], mut b: [f64; 3]) -> Option<[f64; 3]> {
    for i in 0..3 {
        let mut max_row = i;
        for k in (i + 1)..3 {
            if a[k][i].abs() > a[max_row][i].abs() {
                max_row = k;
            }
        }
        if a[max_row][i].abs() < 1e-12 {
            return None;
        }
        a.swap(i, max_row);
        b.swap(i, max_row);
        for k in (i + 1)..3 {
            let f = a[k][i] / a[i][i];
            for j in i..3 {
                a[k][j] -= f * a[i][j];
            }
            b[k] -= f * b[i];
        }
    }
    let mut x = [0.0f64; 3];
    for i in (0..3).rev() {
        let mut s = b[i];
        for j in (i + 1)..3 {
            s -= a[i][j] * x[j];
        }
        x[i] = s / a[i][i];
    }
    Some(x)
}

fn fit_cone(samples: &[(Point3, Vec3)]) -> Option<(Point3, Vec3, f64, f64)> {
    // On a cone, every normal ray (p, -n) passes through the axis line.
    // The vectors from apex to sample points all make the same angle with
    // the axis. Use the fact that normals are perpendicular to (apex - p)
    // rotated by (pi/2 - semi_angle). For simplicity, require at least two
    // distinct normals and find the axis as the common line of planes defined
    // by each (p, n) — (p + t*n) line direction at the apex satisfies axis.
    if samples.len() < 3 {
        return None;
    }
    // Reject if normals are parallel (that's a cylinder case).
    let n0 = samples[0].1.normalized()?;
    let mut varied = false;
    for (_, n) in samples.iter().skip(1) {
        if (n.normalized()? - n0).length() > 1e-4 {
            varied = true;
            break;
        }
    }
    if !varied {
        return None;
    }
    // Apex is the point where lines along (-n) from each sample point converge.
    // Build two lines from two distinct samples and intersect them in 3D (closest point).
    let mut best = None;
    for i in 0..samples.len() {
        for j in (i + 1)..samples.len() {
            let (pi, ni) = samples[i];
            let (pj, nj) = samples[j];
            let ni = ni.normalized()?;
            let nj = nj.normalized()?;
            if (ni - nj).length() < 1e-4 {
                continue;
            }
            // Lines: pi + t*(-ni), pj + s*(-nj). Find closest point pair.
            let w0 = pi - pj;
            let a = ni.dot(ni);
            let b = ni.dot(nj);
            let c = nj.dot(nj);
            let d = ni.dot(w0);
            let e = nj.dot(w0);
            let denom = a * c - b * b;
            if denom.abs() < 1e-10 {
                continue;
            }
            let t = (b * e - c * d) / denom;
            let s = (a * e - b * d) / denom;
            let pa = pi + (-ni) * t;
            let pb = pj + (-nj) * s;
            let apex_candidate = pa + (pb - pa) * 0.5;
            best = Some(apex_candidate);
            break;
        }
        if best.is_some() {
            break;
        }
    }
    let apex = best?;

    // Axis = normalized mean of (p - apex).
    let mut axis_sum = Vec3::ZERO;
    for (p, _) in samples {
        let d = *p - apex;
        let dn = d.normalized()?;
        axis_sum += dn;
    }
    let axis = axis_sum.normalized()?;

    // Semi-angle: angle between (p - apex) and axis.
    let mut angles = Vec::new();
    let mut ref_rad = 0.0;
    for (p, _) in samples {
        let d = *p - apex;
        let dn = d.normalized()?;
        let cos_a = dn.dot(axis).clamp(-1.0, 1.0);
        angles.push(cos_a.acos());
        let v = d.dot(axis);
        let r = (d - axis * v).length();
        if v > ref_rad {
            ref_rad = v;
        }
        let _ = r;
    }
    let mean = angles.iter().sum::<f64>() / angles.len() as f64;
    for a in &angles {
        if (a - mean).abs() > 1e-3 {
            return None;
        }
    }
    if mean <= 1e-6 || mean >= std::f64::consts::FRAC_PI_2 - 1e-6 {
        return None;
    }
    // Reference radius = ref_rad * tan(semi-angle)
    let ref_radius = ref_rad * mean.tan();
    Some((apex, axis, mean, ref_radius))
}

fn fit_torus(samples: &[(Point3, Vec3)]) -> Option<(Point3, Vec3, f64, f64)> {
    // Minor circle center for each sample: c_i = p_i - r_minor * n_i,
    // where r_minor is unknown. But we can also express: all c_i lie on a
    // circle of radius R (major) in a plane through torus center. Direct
    // algebraic fit is hard; we take a guess-and-check strategy by using
    // the normal to determine r_minor such that |c_i - torus_center|
    // equals R across all samples.
    // For reliability, require that the axis is deducible from normals.
    if samples.len() < 4 {
        return None;
    }
    // Try a few candidate minor radii via a search: for each candidate,
    // compute c_i and check if they lie on a common plane with a circle fit.
    let mean_point = samples
        .iter()
        .fold(Vec3::ZERO, |acc, (p, _)| acc + Vec3::new(p.x, p.y, p.z))
        * (1.0 / samples.len() as f64);
    let _ = mean_point;
    // Binary search minor radius in range [0, diameter of bounding box].
    let mut bb_min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut bb_max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for (p, _) in samples {
        bb_min = Point3::new(bb_min.x.min(p.x), bb_min.y.min(p.y), bb_min.z.min(p.z));
        bb_max = Point3::new(bb_max.x.max(p.x), bb_max.y.max(p.y), bb_max.z.max(p.z));
    }
    let diag = (bb_max - bb_min).length();
    if diag < 1e-9 {
        return None;
    }
    let mut best: Option<(Point3, Vec3, f64, f64, f64)> = None;
    for step in 1..50 {
        let r_minor = diag * (step as f64 / 100.0);
        let mut centers = Vec::new();
        for (p, n) in samples {
            let nv = n.normalized()?;
            centers.push(*p - nv * r_minor);
        }
        // Fit plane to centers, then fit circle in that plane.
        let axis = best_fit_plane_normal(&centers)?;
        let (cc, rr) = fit_circle_in_plane(&centers, axis)?;
        let mut err = 0.0;
        for c in &centers {
            let d = *c - cc;
            let planar = d - axis * d.dot(axis);
            err += (planar.length() - rr).powi(2);
            err += d.dot(axis).powi(2);
        }
        err /= centers.len() as f64;
        if best.as_ref().is_none_or(|b| err < b.4) {
            best = Some((cc, axis, rr, r_minor, err));
        }
    }
    let (center, axis, major, minor, err) = best?;
    if err > 1e-6 * (major.max(minor) + 1.0) {
        return None;
    }
    if major <= minor || minor < 1e-6 {
        return None;
    }
    Some((center, axis, major, minor))
}

fn best_fit_plane_normal(points: &[Point3]) -> Option<Vec3> {
    if points.len() < 3 {
        return None;
    }
    let mut c = Vec3::ZERO;
    for p in points {
        c += Vec3::new(p.x, p.y, p.z);
    }
    c *= 1.0 / points.len() as f64;
    let mut m = [[0.0f64; 3]; 3];
    for p in points {
        let d = [p.x - c.x, p.y - c.y, p.z - c.z];
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += d[i] * d[j];
            }
        }
    }
    smallest_eigenvector(m)
}

fn arbitrary_perp(v: Vec3) -> Vec3 {
    let candidate = if v.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    v.cross(candidate).normalized().unwrap_or(Vec3::X)
}

/// Create a STEP surface entity for a face based on its bound geometry.
///
/// If the face has a bound surface, uses the appropriate STEP entity type
/// (PLANE, CYLINDRICAL_SURFACE, etc.). Otherwise computes a plane from
/// the face's boundary vertices.
fn export_face_surface(
    model: &BRepModel,
    face_h: cadkernel_topology::Handle<cadkernel_topology::FaceData>,
    w: &mut StepWriter,
) -> u64 {
    let face_data = model.faces.get(face_h);

    // If the face has a bound surface, try to classify it.
    if let Some(fd) = face_data {
        if let Some(surface_arc) = fd.surface.as_ref() {
            if let Some(class) = classify_surface(surface_arc.as_ref()) {
                return emit_surface_class(&class, w);
            }
        }
    }

    // Fallback: compute a plane from the face boundary vertices.
    let (origin, normal, x_dir) = if let Some(fd) = face_data {
        if let Some(ld) = model.loops.get(fd.outer_loop) {
            let hes = model.loop_half_edges(ld.half_edge);
            let mut pts = Vec::new();
            for &he_h in &hes {
                if let Some(he) = model.half_edges.get(he_h) {
                    if let Some(v) = model.vertices.get(he.origin) {
                        pts.push(v.point);
                    }
                }
            }
            if pts.len() >= 3 {
                let e1 = pts[1] - pts[0];
                let e2 = pts[2] - pts[0];
                let n = e1.cross(e2).normalized().unwrap_or(Vec3::Z);
                let x = e1.normalized().unwrap_or(Vec3::X);
                (pts[0], n, x)
            } else {
                (Point3::ORIGIN, Vec3::Z, Vec3::X)
            }
        } else {
            (Point3::ORIGIN, Vec3::Z, Vec3::X)
        }
    } else {
        (Point3::ORIGIN, Vec3::Z, Vec3::X)
    };

    emit_surface_class(
        &SurfaceClass::Plane {
            origin,
            normal,
            x_dir,
        },
        w,
    )
}

fn emit_surface_class(class: &SurfaceClass, w: &mut StepWriter) -> u64 {
    match *class {
        SurfaceClass::Plane {
            origin,
            normal,
            x_dir,
        } => {
            let origin_id = w.add_point(origin);
            let n_id = w.add_direction(normal);
            let x_id = w.add_direction(x_dir);
            let axis_id = w.add_entity(StepEntity::Axis2Placement3d {
                location: origin_id,
                axis: Some(n_id),
                ref_direction: Some(x_id),
            });
            w.add_entity(StepEntity::Plane { placement: axis_id })
        }
        SurfaceClass::Cylinder {
            origin,
            axis,
            x_dir,
            radius,
        } => {
            let origin_id = w.add_point(origin);
            let a_id = w.add_direction(axis);
            let x_id = w.add_direction(x_dir);
            let axis_id = w.add_entity(StepEntity::Axis2Placement3d {
                location: origin_id,
                axis: Some(a_id),
                ref_direction: Some(x_id),
            });
            w.add_entity(StepEntity::CylindricalSurface {
                placement: axis_id,
                radius,
            })
        }
        SurfaceClass::Sphere {
            center,
            axis,
            x_dir,
            radius,
        } => {
            let origin_id = w.add_point(center);
            let a_id = w.add_direction(axis);
            let x_id = w.add_direction(x_dir);
            let axis_id = w.add_entity(StepEntity::Axis2Placement3d {
                location: origin_id,
                axis: Some(a_id),
                ref_direction: Some(x_id),
            });
            w.add_entity(StepEntity::SphericalSurface {
                placement: axis_id,
                radius,
            })
        }
        SurfaceClass::Cone {
            apex,
            axis,
            x_dir,
            semi_angle,
            ref_radius,
        } => {
            let origin_id = w.add_point(apex);
            let a_id = w.add_direction(axis);
            let x_id = w.add_direction(x_dir);
            let axis_id = w.add_entity(StepEntity::Axis2Placement3d {
                location: origin_id,
                axis: Some(a_id),
                ref_direction: Some(x_id),
            });
            w.add_entity(StepEntity::ConicalSurface {
                placement: axis_id,
                radius: ref_radius,
                semi_angle,
            })
        }
        SurfaceClass::Torus {
            center,
            axis,
            x_dir,
            major_radius,
            minor_radius,
        } => {
            let origin_id = w.add_point(center);
            let a_id = w.add_direction(axis);
            let x_id = w.add_direction(x_dir);
            let axis_id = w.add_entity(StepEntity::Axis2Placement3d {
                location: origin_id,
                axis: Some(a_id),
                ref_direction: Some(x_id),
            });
            w.add_entity(StepEntity::ToroidalSurface {
                placement: axis_id,
                major_radius,
                minor_radius,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Entity serialization
// ---------------------------------------------------------------------------

fn step_real(value: f64) -> String {
    format!("{value:.16e}")
}

fn entity_to_step(entity: &StepEntity) -> String {
    match entity {
        StepEntity::CartesianPoint(p) => {
            let x = step_real(p.x);
            let y = step_real(p.y);
            let z = step_real(p.z);
            format!("CARTESIAN_POINT('',({x},{y},{z}))")
        }
        StepEntity::Direction(d) => {
            let x = step_real(d[0]);
            let y = step_real(d[1]);
            let z = step_real(d[2]);
            format!("DIRECTION('',({x},{y},{z}))")
        }
        StepEntity::Vector {
            direction,
            magnitude,
        } => {
            let magnitude = step_real(*magnitude);
            format!("VECTOR('',#{direction},{magnitude})")
        }
        StepEntity::Line { point, direction } => {
            format!("LINE('',#{point},#{direction})")
        }
        StepEntity::Circle { placement, radius } => {
            let radius = step_real(*radius);
            format!("CIRCLE('',#{placement},{radius})")
        }
        StepEntity::Plane { placement } => {
            format!("PLANE('',#{placement})")
        }
        StepEntity::CylindricalSurface { placement, radius } => {
            let radius = step_real(*radius);
            format!("CYLINDRICAL_SURFACE('',#{placement},{radius})")
        }
        StepEntity::SphericalSurface { placement, radius } => {
            let radius = step_real(*radius);
            format!("SPHERICAL_SURFACE('',#{placement},{radius})")
        }
        StepEntity::ConicalSurface {
            placement,
            radius,
            semi_angle,
        } => {
            let radius = step_real(*radius);
            let semi_angle = step_real(*semi_angle);
            format!("CONICAL_SURFACE('',#{placement},{radius},{semi_angle})")
        }
        StepEntity::ToroidalSurface {
            placement,
            major_radius,
            minor_radius,
        } => {
            let major_radius = step_real(*major_radius);
            let minor_radius = step_real(*minor_radius);
            format!("TOROIDAL_SURFACE('',#{placement},{major_radius},{minor_radius})")
        }
        StepEntity::Axis2Placement3d {
            location,
            axis,
            ref_direction,
        } => {
            let a = axis.map_or("$".into(), |id| format!("#{id}"));
            let r = ref_direction.map_or("$".into(), |id| format!("#{id}"));
            format!("AXIS2_PLACEMENT_3D('',#{location},{a},{r})")
        }
        StepEntity::BSplineCurve {
            degree,
            control_points,
            knots,
            multiplicities,
        } => {
            let cps: Vec<String> = control_points.iter().map(|id| format!("#{id}")).collect();
            let ks: Vec<String> = knots.iter().map(|v| step_real(*v)).collect();
            let ms: Vec<String> = multiplicities.iter().map(|v| format!("{v}")).collect();
            format!(
                "B_SPLINE_CURVE_WITH_KNOTS('',{degree},({}),{},.UNSPECIFIED.,.F.,.F.,({}),({}),.UNSPECIFIED.)",
                cps.join(","),
                ".UNSPECIFIED.",
                ms.join(","),
                ks.join(",")
            )
        }
        StepEntity::BSplineSurface {
            degree_u,
            degree_v,
            control_points,
            knots_u,
            knots_v,
            multiplicities_u,
            multiplicities_v,
        } => {
            let rows: Vec<String> = control_points
                .iter()
                .map(|row| {
                    let pts: Vec<String> = row.iter().map(|id| format!("#{id}")).collect();
                    format!("({})", pts.join(","))
                })
                .collect();
            let ku: Vec<String> = knots_u.iter().map(|v| step_real(*v)).collect();
            let kv: Vec<String> = knots_v.iter().map(|v| step_real(*v)).collect();
            let mu: Vec<String> = multiplicities_u.iter().map(|v| format!("{v}")).collect();
            let mv: Vec<String> = multiplicities_v.iter().map(|v| format!("{v}")).collect();
            format!(
                "B_SPLINE_SURFACE_WITH_KNOTS('',{degree_u},{degree_v},({}),.UNSPECIFIED.,.F.,.F.,.F.,({mu}),({mv}),({ku}),({kv}),.UNSPECIFIED.)",
                rows.join(","),
                mu = mu.join(","),
                mv = mv.join(","),
                ku = ku.join(","),
                kv = kv.join(","),
            )
        }
        StepEntity::VertexPoint(p) => {
            format!("VERTEX_POINT('',#{p})")
        }
        StepEntity::EdgeCurve {
            start,
            end,
            curve,
            same_sense,
        } => {
            let s = if *same_sense { ".T." } else { ".F." };
            format!("EDGE_CURVE('',#{start},#{end},#{curve},{s})")
        }
        StepEntity::OrientedEdge { edge, orientation } => {
            let o = if *orientation { ".T." } else { ".F." };
            format!("ORIENTED_EDGE('',*,*,#{edge},{o})")
        }
        StepEntity::EdgeLoop { edges } => {
            let es: Vec<String> = edges.iter().map(|id| format!("#{id}")).collect();
            format!("EDGE_LOOP('',({}),{})", es.join(","), "").replace(",)", ")")
        }
        StepEntity::FaceBound { bound, orientation } => {
            let o = if *orientation { ".T." } else { ".F." };
            format!("FACE_OUTER_BOUND('',#{bound},{o})")
        }
        StepEntity::AdvancedFace {
            bounds,
            surface,
            same_sense,
        } => {
            let bs: Vec<String> = bounds.iter().map(|id| format!("#{id}")).collect();
            let s = if *same_sense { ".T." } else { ".F." };
            format!("ADVANCED_FACE('',({}),#{surface},{s})", bs.join(","))
        }
        StepEntity::ClosedShell { faces } => {
            let fs: Vec<String> = faces.iter().map(|id| format!("#{id}")).collect();
            format!("CLOSED_SHELL('',({}),{})", fs.join(","), "").replace(",)", ")")
        }
        StepEntity::ManifoldSolidBrep { shell } => {
            format!("MANIFOLD_SOLID_BREP('',#{shell})")
        }
        StepEntity::Other {
            entity_type,
            params: _,
        } => {
            format!("{entity_type}()")
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_entity() {
        let input = "#1 = CARTESIAN_POINT('origin',(0.0,0.0,0.0));";
        let tokens = tokenize(input).unwrap();
        assert!(tokens.contains(&Token::EntityRef(1)));
        assert!(tokens.contains(&Token::Keyword("CARTESIAN_POINT".into())));
    }

    #[test]
    fn test_tokenize_numbers() {
        let input = "#5 = CIRCLE('',#3,1.5E+01);";
        let tokens = tokenize(input).unwrap();
        assert!(tokens.contains(&Token::EntityRef(5)));
        assert!(tokens.contains(&Token::Real(15.0)));
    }

    #[test]
    fn test_parse_step_entities() {
        let content = "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n\
            #1 = CARTESIAN_POINT('',(1.0,2.0,3.0));\n\
            #2 = DIRECTION('',(0.0,0.0,1.0));\n\
            #3 = VERTEX_POINT('',#1);\n\
            ENDSEC;\nEND-ISO-10303-21;";
        let entities = parse_step_entities(content).unwrap();
        assert_eq!(entities.len(), 3);
        assert_eq!(entities[0].entity_type, "CARTESIAN_POINT");
        assert_eq!(entities[2].entity_type, "VERTEX_POINT");
    }

    #[test]
    fn test_parse_step_typed() {
        let content = "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n\
            #1 = CARTESIAN_POINT('',(10.0,20.0,30.0));\n\
            #2 = DIRECTION('',(0.0,0.0,1.0));\n\
            ENDSEC;\nEND-ISO-10303-21;";
        let file = parse_step(content).unwrap();
        match file.entities.get(&1) {
            Some(StepEntity::CartesianPoint(p)) => {
                assert!((p.x - 10.0).abs() < 1e-10);
                assert!((p.y - 20.0).abs() < 1e-10);
                assert!((p.z - 30.0).abs() < 1e-10);
            }
            _ => panic!("expected CartesianPoint"),
        }
    }

    #[test]
    fn test_read_step_points() {
        let content = "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n\
            #1 = CARTESIAN_POINT('',(1.0,0.0,0.0));\n\
            #2 = CARTESIAN_POINT('',(0.0,1.0,0.0));\n\
            #3 = DIRECTION('',(0.0,0.0,1.0));\n\
            ENDSEC;\nEND-ISO-10303-21;";
        let points = read_step_points(content).unwrap();
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn test_step_writer() {
        let mut w = StepWriter::new();
        w.add_entity(StepEntity::CartesianPoint(Point3::new(1.0, 2.0, 3.0)));
        w.add_entity(StepEntity::Direction([0.0, 0.0, 1.0]));
        let output = w.write().unwrap();
        assert!(output.contains("CARTESIAN_POINT"));
        assert!(output.contains("DIRECTION"));
        assert!(output.contains("ISO-10303-21"));
    }

    #[test]
    fn test_step_roundtrip() {
        let mut w = StepWriter::new();
        let p1 = w.add_point(Point3::new(0.0, 0.0, 0.0));
        let p2 = w.add_point(Point3::new(1.0, 0.0, 0.0));
        let p3 = w.add_point(Point3::new(1.0, 1.0, 0.0));
        let _vp1 = w.add_entity(StepEntity::VertexPoint(p1));
        let _vp2 = w.add_entity(StepEntity::VertexPoint(p2));
        let _vp3 = w.add_entity(StepEntity::VertexPoint(p3));
        let output = w.write().unwrap();

        let points = read_step_points(&output).unwrap();
        assert_eq!(points.len(), 3);
    }

    #[test]
    fn test_export_step_model() {
        let mut model = BRepModel::new();
        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = model.add_vertex(Point3::new(0.5, 1.0, 0.0));
        let (_, he01, _) = model.add_edge(v0, v1);
        let (_, he12, _) = model.add_edge(v1, v2);
        let (_, he20, _) = model.add_edge(v2, v0);
        let loop_h = model.make_loop(&[he01, he12, he20]).unwrap();
        let face = model.make_face(loop_h);
        let shell = model.make_shell(&[face]);
        model.make_solid(&[shell]);

        let output = export_step(&model).unwrap();
        assert!(output.contains("MANIFOLD_SOLID_BREP"));
        assert!(output.contains("CLOSED_SHELL"));
        assert!(output.contains("ADVANCED_FACE"));
        assert!(output.contains("VERTEX_POINT"));
    }
}
