use cadkernel_core::{KernelError, KernelResult};

// ---------------------------------------------------------------------------
// KernelError construction
// ---------------------------------------------------------------------------

#[test]
fn construct_invalid_handle() {
    let err = KernelError::InvalidHandle("vertex");
    assert!(matches!(err, KernelError::InvalidHandle("vertex")));
}

#[test]
fn construct_invalid_argument() {
    let err = KernelError::InvalidArgument("radius must be positive".into());
    assert!(matches!(err, KernelError::InvalidArgument(_)));
}

#[test]
fn construct_validation_failed() {
    let err = KernelError::ValidationFailed("non-manifold edge".into());
    assert!(matches!(err, KernelError::ValidationFailed(_)));
}

#[test]
fn construct_topology_error() {
    let err = KernelError::TopologyError("dangling half-edge".into());
    assert!(matches!(err, KernelError::TopologyError(_)));
}

#[test]
fn construct_geometry_error() {
    let err = KernelError::GeometryError("zero-length vector".into());
    assert!(matches!(err, KernelError::GeometryError(_)));
}

#[test]
fn construct_io_error() {
    let err = KernelError::IoError("file not found".into());
    assert!(matches!(err, KernelError::IoError(_)));
}

// ---------------------------------------------------------------------------
// Display formatting
// ---------------------------------------------------------------------------

#[test]
fn display_invalid_handle_contains_entity_name() {
    let msg = KernelError::InvalidHandle("face").to_string();
    assert!(msg.contains("face"), "entity name missing from: {msg}");
    assert!(msg.contains("invalid handle"), "prefix missing from: {msg}");
}

#[test]
fn display_invalid_handle_mentions_deleted() {
    let msg = KernelError::InvalidHandle("shell").to_string();
    assert!(
        msg.contains("deleted") || msg.contains("another model"),
        "should explain possible cause: {msg}"
    );
}

#[test]
fn display_invalid_argument_contains_message() {
    let msg = KernelError::InvalidArgument("segments must be >= 3".into()).to_string();
    assert!(msg.contains("invalid argument"), "prefix missing: {msg}");
    assert!(msg.contains("segments"), "detail missing: {msg}");
}

#[test]
fn display_validation_failed_contains_message() {
    let msg = KernelError::ValidationFailed("open shell".into()).to_string();
    assert!(msg.contains("validation failed"), "prefix missing: {msg}");
    assert!(msg.contains("open shell"), "detail missing: {msg}");
}

#[test]
fn display_topology_error_contains_message() {
    let msg = KernelError::TopologyError("loop fewer than 3 edges".into()).to_string();
    assert!(msg.contains("topology error"), "prefix missing: {msg}");
    assert!(msg.contains("loop"), "detail missing: {msg}");
}

#[test]
fn display_geometry_error_contains_message() {
    let msg = KernelError::GeometryError("singular matrix".into()).to_string();
    assert!(msg.contains("geometry error"), "prefix missing: {msg}");
    assert!(msg.contains("singular"), "detail missing: {msg}");
}

#[test]
fn display_io_error_contains_message() {
    let msg = KernelError::IoError("permission denied".into()).to_string();
    assert!(msg.contains("I/O error"), "prefix missing: {msg}");
    assert!(msg.contains("permission denied"), "detail missing: {msg}");
}

// ---------------------------------------------------------------------------
// is_* predicate methods
// ---------------------------------------------------------------------------

#[test]
fn predicate_is_invalid_handle_true_only_for_invalid_handle() {
    assert!(KernelError::InvalidHandle("v").is_invalid_handle());
    assert!(!KernelError::InvalidArgument("x".into()).is_invalid_handle());
    assert!(!KernelError::ValidationFailed("x".into()).is_invalid_handle());
    assert!(!KernelError::TopologyError("x".into()).is_invalid_handle());
    assert!(!KernelError::GeometryError("x".into()).is_invalid_handle());
    assert!(!KernelError::IoError("x".into()).is_invalid_handle());
}

#[test]
fn predicate_is_invalid_argument_true_only_for_invalid_argument() {
    assert!(KernelError::InvalidArgument("x".into()).is_invalid_argument());
    assert!(!KernelError::InvalidHandle("v").is_invalid_argument());
    assert!(!KernelError::ValidationFailed("x".into()).is_invalid_argument());
    assert!(!KernelError::IoError("x".into()).is_invalid_argument());
}

#[test]
fn predicate_is_io_error_true_only_for_io_error() {
    assert!(KernelError::IoError("x".into()).is_io_error());
    assert!(!KernelError::InvalidHandle("v").is_io_error());
    assert!(!KernelError::InvalidArgument("x".into()).is_io_error());
    assert!(!KernelError::GeometryError("x".into()).is_io_error());
}

// ---------------------------------------------------------------------------
// with_context wrapping
// ---------------------------------------------------------------------------

#[test]
fn with_context_prepends_to_invalid_argument() {
    let err = KernelError::InvalidArgument("radius must be > 0".into());
    let wrapped = err.with_context("make_cylinder");
    let msg = wrapped.to_string();
    assert!(msg.contains("make_cylinder"), "context missing: {msg}");
    assert!(
        msg.contains("radius must be > 0"),
        "original missing: {msg}"
    );
}

#[test]
fn with_context_prepends_to_validation_failed() {
    let err = KernelError::ValidationFailed("non-manifold".into());
    let wrapped = err.with_context("check_geometry");
    let msg = wrapped.to_string();
    assert!(msg.contains("check_geometry"), "context missing: {msg}");
    assert!(msg.contains("non-manifold"), "original missing: {msg}");
}

#[test]
fn with_context_prepends_to_topology_error() {
    let err = KernelError::TopologyError("broken loop".into());
    let wrapped = err.with_context("extrude");
    let msg = wrapped.to_string();
    assert!(msg.contains("extrude"), "context missing: {msg}");
    assert!(msg.contains("broken loop"), "original missing: {msg}");
}

#[test]
fn with_context_prepends_to_geometry_error() {
    let err = KernelError::GeometryError("degenerate surface".into());
    let wrapped = err.with_context("nurbs_eval");
    let msg = wrapped.to_string();
    assert!(msg.contains("nurbs_eval"), "context missing: {msg}");
    assert!(
        msg.contains("degenerate surface"),
        "original missing: {msg}"
    );
}

#[test]
fn with_context_prepends_to_io_error() {
    let err = KernelError::IoError("disk full".into());
    let wrapped = err.with_context("export_step");
    let msg = wrapped.to_string();
    assert!(msg.contains("export_step"), "context missing: {msg}");
    assert!(msg.contains("disk full"), "original missing: {msg}");
}

#[test]
fn with_context_invalid_handle_passes_through_unchanged() {
    let err = KernelError::InvalidHandle("edge");
    let wrapped = err.with_context("any_op");
    // InvalidHandle carries a &'static str; context is not concatenated.
    assert!(
        matches!(wrapped, KernelError::InvalidHandle("edge")),
        "handle entity name should be preserved"
    );
}

#[test]
fn with_context_preserves_variant_type_for_all_string_variants() {
    let cases: &[(&str, KernelError)] = &[
        ("op", KernelError::InvalidArgument("bad".into())),
        ("op", KernelError::ValidationFailed("bad".into())),
        ("op", KernelError::TopologyError("bad".into())),
        ("op", KernelError::GeometryError("bad".into())),
        ("op", KernelError::IoError("bad".into())),
    ];
    for (ctx, err) in cases {
        let original_discriminant = std::mem::discriminant(err);
        let wrapped = err.clone().with_context(ctx);
        assert_eq!(
            std::mem::discriminant(&wrapped),
            original_discriminant,
            "variant changed after with_context"
        );
    }
}

// ---------------------------------------------------------------------------
// KernelResult usage patterns
// ---------------------------------------------------------------------------

#[test]
fn kernel_result_ok_passes_through() {
    fn make_result() -> KernelResult<f64> {
        Ok(1.5)
    }
    let result = make_result();
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!((val - 1.5).abs() < f64::EPSILON);
}

#[test]
fn kernel_result_err_carries_error() {
    fn make_err() -> KernelResult<()> {
        Err(KernelError::InvalidArgument("test".into()))
    }
    let result = make_err();
    assert!(result.is_err());
    assert!(result.unwrap_err().is_invalid_argument());
}

#[test]
fn kernel_result_question_mark_propagates() {
    fn inner() -> KernelResult<f64> {
        Err(KernelError::GeometryError("zero det".into()))
    }
    fn outer() -> KernelResult<f64> {
        let v = inner()?;
        Ok(v * 2.0)
    }
    let r = outer();
    assert!(r.is_err());
    let msg = r.unwrap_err().to_string();
    assert!(msg.contains("zero det"));
}

#[test]
fn kernel_result_map_transforms_ok_value() {
    let result: KernelResult<i32> = Ok(5);
    let doubled = result.map(|v| v * 2);
    assert_eq!(doubled.unwrap(), 10);
}

#[test]
fn kernel_result_map_err_transforms_error() {
    let result: KernelResult<()> = Err(KernelError::IoError("raw".into()));
    let mapped = result.map_err(|e| e.with_context("upload"));
    let msg = mapped.unwrap_err().to_string();
    assert!(msg.contains("upload"), "context not added: {msg}");
    assert!(msg.contains("raw"), "original missing: {msg}");
}

// ---------------------------------------------------------------------------
// From<std::io::Error> conversion
// ---------------------------------------------------------------------------

#[test]
fn from_std_io_not_found_converts_to_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
    let kernel_err = KernelError::from(io_err);
    assert!(kernel_err.is_io_error());
    assert!(kernel_err.to_string().contains("no such file"));
}

#[test]
fn from_std_io_permission_denied_converts_to_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    let kernel_err: KernelError = io_err.into();
    assert!(kernel_err.is_io_error());
    assert!(kernel_err.to_string().contains("access denied"));
}

#[test]
fn from_std_io_via_question_mark_in_kernel_result() {
    fn read_something() -> KernelResult<Vec<u8>> {
        let _bytes = std::fs::read("/this/path/does/not/exist")?;
        Ok(vec![])
    }
    let r = read_something();
    assert!(r.is_err());
    assert!(r.unwrap_err().is_io_error());
}

// ---------------------------------------------------------------------------
// std::error::Error trait impl
// ---------------------------------------------------------------------------

#[test]
fn std_error_trait_display_matches_to_string() {
    let err = KernelError::TopologyError("broken edge".into());
    let std_err: &dyn std::error::Error = &err;
    assert_eq!(std_err.to_string(), err.to_string());
}

#[test]
fn std_error_no_source_by_default() {
    let err = KernelError::ValidationFailed("open boundary".into());
    let std_err: &dyn std::error::Error = &err;
    assert!(std_err.source().is_none());
}

// ---------------------------------------------------------------------------
// Clone and PartialEq
// ---------------------------------------------------------------------------

#[test]
fn kernel_error_clone_equals_original() {
    let err = KernelError::InvalidArgument("cloned value".into());
    let cloned = err.clone();
    assert_eq!(err, cloned);
}

#[test]
fn kernel_error_different_variants_not_equal() {
    let a = KernelError::InvalidArgument("x".into());
    let b = KernelError::IoError("x".into());
    assert_ne!(a, b);
}

#[test]
fn kernel_error_same_variant_same_message_equals() {
    let a = KernelError::GeometryError("degenerate".into());
    let b = KernelError::GeometryError("degenerate".into());
    assert_eq!(a, b);
}

#[test]
fn kernel_error_same_variant_different_message_not_equal() {
    let a = KernelError::ValidationFailed("msg A".into());
    let b = KernelError::ValidationFailed("msg B".into());
    assert_ne!(a, b);
}

// ---------------------------------------------------------------------------
// Send + Sync
// ---------------------------------------------------------------------------

#[test]
fn kernel_error_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<KernelError>();
}

#[test]
fn kernel_result_unit_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<KernelResult<()>>();
}
