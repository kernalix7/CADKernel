use cadkernel_api::{ApiError, ApiResult, Command, Document, Outcome, Session};

#[test]
fn stable_entry_points_remain_public() {
    let anchors = [
        std::any::type_name::<Document>(),
        std::any::type_name::<Command>(),
        std::any::type_name::<Outcome>(),
        std::any::type_name::<Session>(),
        std::any::type_name::<ApiError>(),
    ];

    for anchor in anchors {
        assert!(anchor.starts_with("cadkernel_api::"));
    }

    let ok: ApiResult<()> = Ok(());
    assert!(ok.is_ok());

    let err = ApiError::InvalidArgument("radius must be positive".to_owned());
    assert_eq!(err.to_string(), "invalid argument: radius must be positive");
}
