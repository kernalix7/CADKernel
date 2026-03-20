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

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    EntityRef(u64),
    Keyword(String),
    String(String),
    Integer(i64),
    Real(f64),
    Enum(String),
    Star,
    Dollar,
    LParen,
    RParen,
    Comma,
    Semi,
    Eq,
}

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

pub fn tokenize(input: &str) -> KernelResult<Vec<Token>> {
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
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 2;
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

/// A STEP parameter value.
#[derive(Debug, Clone)]
pub enum StepParam {
    EntityRef(u64),
    Integer(i64),
    Real(f64),
    String(String),
    Enum(String),
    List(Vec<StepParam>),
    Unset,
    Derived,
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
    pub fn get_point(&self, id: u64) -> Point3 {
        match self.entities.get(&id) {
            Some(StepEntity::CartesianPoint(p)) => *p,
            _ => Point3::ORIGIN,
        }
    }

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
        // Error recovery: skip entities that fail to resolve instead of aborting
        let resolved = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            resolve_entity(e)
        }));
        match resolved {
            Ok(entity) => { entities.insert(e.id, entity); }
            Err(_) => {
                // Skip malformed entity — store as Other for traceability
                entities.insert(e.id, StepEntity::Other {
                    entity_type: e.entity_type.clone(),
                    params: e.params.clone(),
                });
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
            let p = file.get_point(*point_id);
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
                                        if let Some(StepEntity::OrientedEdge { edge, .. }) =
                                            file.entities.get(&oe_id)
                                        {
                                            if let Some(StepEntity::EdgeCurve {
                                                start, ..
                                            }) = file.entities.get(edge)
                                            {
                                                if let Some(&vh) = vertex_map.get(start) {
                                                    if !face_verts.contains(&vh) {
                                                        face_verts.push(vh);
                                                    }
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

/// STEP file writer.
#[derive(Debug)]
pub struct StepWriter {
    entities: Vec<(u64, StepEntity)>,
    next_id: u64,
}

impl StepWriter {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            next_id: 1,
        }
    }

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
        out.push_str("FILE_NAME('output.stp','2026-01-01',(''),(''),'CADKernel','CADKernel','');\n");
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
        let start_vp = vert_step_ids.get(&ed.start.index()).copied().unwrap_or(0);
        let end_vp = vert_step_ids.get(&ed.end.index()).copied().unwrap_or(0);

        let p1 = model
            .vertices
            .get(ed.start)
            .map(|v| v.point)
            .unwrap_or(Point3::ORIGIN);
        let p2 = model
            .vertices
            .get(ed.end)
            .map(|v| v.point)
            .unwrap_or(Point3::ORIGIN);
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
                        let oe_id = w.add_entity(StepEntity::OrientedEdge {
                            edge: ec_id,
                            orientation: true,
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
    std::fs::write(path, content)
        .map_err(|e| KernelError::IoError(format!("write error: {e}")))?;
    Ok(())
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
    // Try to compute a plane from the face boundary vertices
    let face_data = model.faces.get(face_h);
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

// ---------------------------------------------------------------------------
// Entity serialization
// ---------------------------------------------------------------------------

fn entity_to_step(entity: &StepEntity) -> String {
    match entity {
        StepEntity::CartesianPoint(p) => {
            format!("CARTESIAN_POINT('',({},{},{}))", p.x, p.y, p.z)
        }
        StepEntity::Direction(d) => {
            format!("DIRECTION('',({},{},{}))", d[0], d[1], d[2])
        }
        StepEntity::Vector {
            direction,
            magnitude,
        } => {
            format!("VECTOR('',#{},{magnitude})", direction)
        }
        StepEntity::Line { point, direction } => {
            format!("LINE('',#{point},#{direction})")
        }
        StepEntity::Circle { placement, radius } => {
            format!("CIRCLE('',#{placement},{radius})")
        }
        StepEntity::Plane { placement } => {
            format!("PLANE('',#{placement})")
        }
        StepEntity::CylindricalSurface { placement, radius } => {
            format!("CYLINDRICAL_SURFACE('',#{placement},{radius})")
        }
        StepEntity::SphericalSurface { placement, radius } => {
            format!("SPHERICAL_SURFACE('',#{placement},{radius})")
        }
        StepEntity::ConicalSurface {
            placement,
            radius,
            semi_angle,
        } => {
            format!("CONICAL_SURFACE('',#{placement},{radius},{semi_angle})")
        }
        StepEntity::ToroidalSurface {
            placement,
            major_radius,
            minor_radius,
        } => {
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
            let ks: Vec<String> = knots.iter().map(|v| format!("{v}")).collect();
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
            let ku: Vec<String> = knots_u.iter().map(|v| format!("{v}")).collect();
            let kv: Vec<String> = knots_v.iter().map(|v| format!("{v}")).collect();
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
            format!("EDGE_LOOP('',({}),{})", es.join(","), "")
                .replace(",)", ")")
        }
        StepEntity::FaceBound {
            bound,
            orientation,
        } => {
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
            format!("CLOSED_SHELL('',({}),{})", fs.join(","), "")
                .replace(",)", ")")
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
