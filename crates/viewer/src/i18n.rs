#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Language {
    #[default]
    En,
    Ko,
}

impl Language {
    pub const ALL: &[Language] = &[Language::En, Language::Ko];

    pub fn label(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Ko => "Korean",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Translation {
    pub key: &'static str,
    pub en: &'static str,
    pub ko: &'static str,
}

pub const TRANSLATIONS: &[Translation] = &[
    Translation {
        key: "app.name",
        en: "CADKernel",
        ko: "CADKernel",
    },
    Translation {
        key: "app.subtitle",
        en: "Open-source CAD Software",
        ko: "오픈소스 CAD 소프트웨어",
    },
    Translation {
        key: "app.ready",
        en: "Ready",
        ko: "준비됨",
    },
    Translation {
        key: "menu.file",
        en: "File",
        ko: "파일",
    },
    Translation {
        key: "menu.file.new",
        en: "New",
        ko: "새로 만들기",
    },
    Translation {
        key: "menu.file.new_tab",
        en: "New Tab",
        ko: "새 탭",
    },
    Translation {
        key: "menu.file.close_tab",
        en: "Close Tab",
        ko: "탭 닫기",
    },
    Translation {
        key: "menu.file.open",
        en: "Open...",
        ko: "열기...",
    },
    Translation {
        key: "menu.file.save_as",
        en: "Save As...",
        ko: "다른 이름으로 저장...",
    },
    Translation {
        key: "menu.file.recent",
        en: "Recent Files",
        ko: "최근 파일",
    },
    Translation {
        key: "menu.file.clear_recent",
        en: "Clear Recent Files",
        ko: "최근 파일 지우기",
    },
    Translation {
        key: "menu.file.inspect_cadk",
        en: "Inspect Command File...",
        ko: "명령 파일 검사...",
    },
    Translation {
        key: "menu.file.import",
        en: "Import",
        ko: "가져오기",
    },
    Translation {
        key: "menu.file.export",
        en: "Export",
        ko: "내보내기",
    },
    Translation {
        key: "menu.edit",
        en: "Edit",
        ko: "편집",
    },
    Translation {
        key: "menu.edit.undo",
        en: "Undo",
        ko: "실행 취소",
    },
    Translation {
        key: "menu.edit.redo",
        en: "Redo",
        ko: "다시 실행",
    },
    Translation {
        key: "menu.edit.select_all",
        en: "Select All",
        ko: "모두 선택",
    },
    Translation {
        key: "menu.edit.deselect_all",
        en: "Deselect All",
        ko: "선택 해제",
    },
    Translation {
        key: "menu.edit.delete",
        en: "Delete",
        ko: "삭제",
    },
    Translation {
        key: "menu.edit.groups",
        en: "Groups",
        ko: "그룹",
    },
    Translation {
        key: "menu.edit.settings",
        en: "Settings...",
        ko: "설정...",
    },
    Translation {
        key: "menu.create",
        en: "Create",
        ko: "생성",
    },
    Translation {
        key: "menu.create.box",
        en: "Box",
        ko: "박스",
    },
    Translation {
        key: "menu.create.cylinder",
        en: "Cylinder",
        ko: "원통",
    },
    Translation {
        key: "menu.create.sphere",
        en: "Sphere",
        ko: "구",
    },
    Translation {
        key: "menu.create.cone",
        en: "Cone",
        ko: "원뿔",
    },
    Translation {
        key: "menu.create.torus",
        en: "Torus",
        ko: "토러스",
    },
    Translation {
        key: "menu.create.tube",
        en: "Tube",
        ko: "튜브",
    },
    Translation {
        key: "menu.create.prism",
        en: "Prism",
        ko: "프리즘",
    },
    Translation {
        key: "menu.create.wedge",
        en: "Wedge",
        ko: "쐐기",
    },
    Translation {
        key: "menu.create.ellipsoid",
        en: "Ellipsoid",
        ko: "타원체",
    },
    Translation {
        key: "menu.create.helix",
        en: "Helix",
        ko: "나선",
    },
    Translation {
        key: "menu.macro",
        en: "Macro",
        ko: "매크로",
    },
    Translation {
        key: "menu.view",
        en: "View",
        ko: "보기",
    },
    Translation {
        key: "menu.view.panels",
        en: "Panels",
        ko: "패널",
    },
    Translation {
        key: "menu.view.display_mode",
        en: "Display Mode",
        ko: "표시 모드",
    },
    Translation {
        key: "menu.view.standard_views",
        en: "Standard Views",
        ko: "표준 뷰",
    },
    Translation {
        key: "menu.view.bookmarks",
        en: "Bookmarks",
        ko: "북마크",
    },
    Translation {
        key: "menu.workbench",
        en: "Workbench",
        ko: "워크벤치",
    },
    Translation {
        key: "menu.tools",
        en: "Tools",
        ko: "도구",
    },
    Translation {
        key: "menu.tools.scripting",
        en: "Scripting",
        ko: "스크립팅",
    },
    Translation {
        key: "menu.tools.plugins",
        en: "Plugins",
        ko: "플러그인",
    },
    Translation {
        key: "menu.tools.mcp",
        en: "MCP",
        ko: "MCP",
    },
    Translation {
        key: "menu.help",
        en: "Help",
        ko: "도움말",
    },
    Translation {
        key: "menu.part",
        en: "Part",
        ko: "파트",
    },
    Translation {
        key: "menu.part.primitives",
        en: "Primitives",
        ko: "기본 형상",
    },
    Translation {
        key: "menu.part.boolean",
        en: "Boolean",
        ko: "불리언",
    },
    Translation {
        key: "menu.part.join",
        en: "Join",
        ko: "결합",
    },
    Translation {
        key: "menu.part.compound",
        en: "Compound",
        ko: "복합체",
    },
    Translation {
        key: "menu.part.transform",
        en: "Transform",
        ko: "변환",
    },
    Translation {
        key: "menu.part.convert",
        en: "Convert",
        ko: "변환",
    },
    Translation {
        key: "menu.part.features",
        en: "Features",
        ko: "피처",
    },
    Translation {
        key: "menu.partdesign",
        en: "PartDesign",
        ko: "파트 디자인",
    },
    Translation {
        key: "menu.partdesign.additive",
        en: "Additive",
        ko: "더하기",
    },
    Translation {
        key: "menu.partdesign.subtractive",
        en: "Subtractive",
        ko: "빼기",
    },
    Translation {
        key: "menu.partdesign.features",
        en: "Features",
        ko: "피처",
    },
    Translation {
        key: "menu.partdesign.pattern",
        en: "Pattern",
        ko: "패턴",
    },
    Translation {
        key: "menu.partdesign.mechanical",
        en: "Mechanical",
        ko: "기계 요소",
    },
    Translation {
        key: "menu.partdesign.body",
        en: "Body",
        ko: "바디",
    },
    Translation {
        key: "menu.sketch",
        en: "Sketch",
        ko: "스케치",
    },
    Translation {
        key: "menu.sketch.geometry",
        en: "Geometry",
        ko: "도형",
    },
    Translation {
        key: "menu.sketch.constraints",
        en: "Constraints",
        ko: "구속",
    },
    Translation {
        key: "menu.sketch.tools",
        en: "Tools",
        ko: "도구",
    },
    Translation {
        key: "menu.sketch.bspline",
        en: "B-Spline",
        ko: "B-스플라인",
    },
    Translation {
        key: "menu.sketch.toggles",
        en: "Toggles",
        ko: "토글",
    },
    Translation {
        key: "menu.mesh",
        en: "Mesh",
        ko: "메시",
    },
    Translation {
        key: "menu.mesh.import_export",
        en: "Import/Export",
        ko: "가져오기/내보내기",
    },
    Translation {
        key: "menu.mesh.modify",
        en: "Modify",
        ko: "수정",
    },
    Translation {
        key: "menu.mesh.analyze",
        en: "Analyze",
        ko: "분석",
    },
    Translation {
        key: "menu.techdraw",
        en: "TechDraw",
        ko: "테크드로우",
    },
    Translation {
        key: "menu.techdraw.page",
        en: "Page",
        ko: "페이지",
    },
    Translation {
        key: "menu.techdraw.views",
        en: "Views",
        ko: "뷰",
    },
    Translation {
        key: "menu.techdraw.dimensions",
        en: "Dimensions",
        ko: "치수",
    },
    Translation {
        key: "menu.techdraw.annotations",
        en: "Annotations",
        ko: "주석",
    },
    Translation {
        key: "menu.techdraw.centerlines",
        en: "Centerlines",
        ko: "중심선",
    },
    Translation {
        key: "menu.techdraw.export",
        en: "Export",
        ko: "내보내기",
    },
    Translation {
        key: "menu.assembly",
        en: "Assembly",
        ko: "어셈블리",
    },
    Translation {
        key: "menu.assembly.joints",
        en: "Joints",
        ko: "조인트",
    },
    Translation {
        key: "menu.draft",
        en: "Draft",
        ko: "드래프트",
    },
    Translation {
        key: "menu.draft.drawing",
        en: "Drawing",
        ko: "드로잉",
    },
    Translation {
        key: "menu.draft.modification",
        en: "Modification",
        ko: "수정",
    },
    Translation {
        key: "menu.draft.arrays",
        en: "Arrays",
        ko: "배열",
    },
    Translation {
        key: "menu.draft.annotation",
        en: "Annotation",
        ko: "주석",
    },
    Translation {
        key: "menu.draft.conversion",
        en: "Conversion",
        ko: "변환",
    },
    Translation {
        key: "menu.draft.snap",
        en: "Snap",
        ko: "스냅",
    },
    Translation {
        key: "menu.surface",
        en: "Surface",
        ko: "서피스",
    },
    Translation {
        key: "menu.fem",
        en: "FEM",
        ko: "FEM",
    },
    Translation {
        key: "menu.fem.analysis",
        en: "Analysis",
        ko: "해석",
    },
    Translation {
        key: "menu.fem.material",
        en: "Material",
        ko: "재질",
    },
    Translation {
        key: "menu.fem.boundary",
        en: "Boundary Conditions",
        ko: "경계 조건",
    },
    Translation {
        key: "menu.fem.loads",
        en: "Loads",
        ko: "하중",
    },
    Translation {
        key: "menu.fem.constraints",
        en: "Constraints",
        ko: "구속",
    },
    Translation {
        key: "menu.fem.thermal",
        en: "Thermal",
        ko: "열",
    },
    Translation {
        key: "menu.fem.mesh",
        en: "Mesh",
        ko: "메시",
    },
    Translation {
        key: "menu.fem.solve",
        en: "Solve",
        ko: "해석 실행",
    },
    Translation {
        key: "menu.fem.results",
        en: "Results",
        ko: "결과",
    },
    Translation {
        key: "dialog.autosave.title",
        en: "Recover Autosave",
        ko: "자동 저장 복구",
    },
    Translation {
        key: "dialog.autosave.message",
        en: "Autosave snapshots were found.",
        ko: "자동 저장 스냅샷을 찾았습니다.",
    },
    Translation {
        key: "dialog.autosave.recover",
        en: "Recover",
        ko: "복구",
    },
    Translation {
        key: "dialog.autosave.discard",
        en: "Discard",
        ko: "삭제",
    },
    Translation {
        key: "dialog.autosave.skip",
        en: "Skip",
        ko: "건너뛰기",
    },
    Translation {
        key: "dialog.autosave.thumbnail",
        en: "Thumbnail",
        ko: "썸네일",
    },
    Translation {
        key: "dialog.autosave.no_thumbnail",
        en: "No thumbnail",
        ko: "썸네일 없음",
    },
    Translation {
        key: "dialog.close_tab.title",
        en: "Close Document",
        ko: "문서 닫기",
    },
    Translation {
        key: "dialog.close_tab.message",
        en: "This document has unsaved changes.",
        ko: "이 문서에 저장되지 않은 변경 사항이 있습니다.",
    },
    Translation {
        key: "dialog.close_tab.close",
        en: "Close",
        ko: "닫기",
    },
    Translation {
        key: "dialog.close_tab.cancel",
        en: "Cancel",
        ko: "취소",
    },
    Translation {
        key: "dialog.settings.title",
        en: "Preferences",
        ko: "환경설정",
    },
    Translation {
        key: "settings.general",
        en: "General",
        ko: "일반",
    },
    Translation {
        key: "settings.display",
        en: "Display",
        ko: "표시",
    },
    Translation {
        key: "settings.navigation",
        en: "Navigation",
        ko: "탐색",
    },
    Translation {
        key: "settings.appearance",
        en: "Appearance",
        ko: "모양",
    },
    Translation {
        key: "settings.lighting",
        en: "Lighting",
        ko: "조명",
    },
    Translation {
        key: "settings.shortcuts",
        en: "Shortcuts",
        ko: "단축키",
    },
    Translation {
        key: "settings.reset_all",
        en: "Reset All",
        ko: "모두 초기화",
    },
    Translation {
        key: "settings.units",
        en: "Units",
        ko: "단위",
    },
    Translation {
        key: "settings.unit_system",
        en: "Unit system:",
        ko: "단위계:",
    },
    Translation {
        key: "settings.decimal_places",
        en: "Decimal places:",
        ko: "소수 자릿수:",
    },
    Translation {
        key: "settings.files",
        en: "Files",
        ko: "파일",
    },
    Translation {
        key: "settings.enable_autosave",
        en: "Enable auto-save",
        ko: "자동 저장 사용",
    },
    Translation {
        key: "settings.interval",
        en: "Interval:",
        ko: "간격:",
    },
    Translation {
        key: "settings.recent_limit",
        en: "Recent files limit:",
        ko: "최근 파일 제한:",
    },
    Translation {
        key: "settings.behavior",
        en: "Behavior",
        ko: "동작",
    },
    Translation {
        key: "settings.confirm_delete",
        en: "Confirm before deleting objects",
        ko: "객체 삭제 전 확인",
    },
    Translation {
        key: "settings.language",
        en: "Language",
        ko: "언어",
    },
    Translation {
        key: "settings.theme",
        en: "Theme",
        ko: "테마",
    },
    Translation {
        key: "settings.theme.dark",
        en: "Dark",
        ko: "어둡게",
    },
    Translation {
        key: "settings.theme.light",
        en: "Light",
        ko: "밝게",
    },
    Translation {
        key: "settings.theme.system",
        en: "System",
        ko: "시스템",
    },
    Translation {
        key: "settings.ui_density",
        en: "UI Density",
        ko: "UI 밀도",
    },
    Translation {
        key: "settings.density.compact",
        en: "Compact",
        ko: "조밀하게",
    },
    Translation {
        key: "settings.density.normal",
        en: "Normal",
        ko: "보통",
    },
    Translation {
        key: "settings.density.spacious",
        en: "Spacious",
        ko: "넓게",
    },
    Translation {
        key: "settings.background",
        en: "Background",
        ko: "배경",
    },
    Translation {
        key: "settings.viewport_overlays",
        en: "Viewport Overlays",
        ko: "뷰포트 오버레이",
    },
    Translation {
        key: "settings.camera",
        en: "Camera",
        ko: "카메라",
    },
    Translation {
        key: "settings.selection",
        en: "Selection",
        ko: "선택",
    },
    Translation {
        key: "settings.tessellation",
        en: "Tessellation",
        ko: "테셀레이션",
    },
    Translation {
        key: "settings.section_plane",
        en: "Section Plane",
        ko: "단면 평면",
    },
    Translation {
        key: "settings.mouse_style",
        en: "Mouse Style",
        ko: "마우스 스타일",
    },
    Translation {
        key: "settings.orbit_rotation",
        en: "Orbit & Rotation",
        ko: "궤도 회전",
    },
    Translation {
        key: "settings.sensitivity",
        en: "Sensitivity",
        ko: "감도",
    },
    Translation {
        key: "settings.animation",
        en: "Animation",
        ko: "애니메이션",
    },
    Translation {
        key: "settings.view_cube",
        en: "View Cube",
        ko: "뷰 큐브",
    },
    Translation {
        key: "settings.scene_lighting",
        en: "Scene Lighting",
        ko: "장면 조명",
    },
    Translation {
        key: "status.new_model",
        en: "New model",
        ko: "새 모델",
    },
    Translation {
        key: "status.new_tab",
        en: "New tab",
        ko: "새 탭",
    },
    Translation {
        key: "status.close_tab",
        en: "Closed tab",
        ko: "탭을 닫음",
    },
    Translation {
        key: "status.opening",
        en: "Opening",
        ko: "여는 중",
    },
    Translation {
        key: "status.loading",
        en: "Loading",
        ko: "불러오는 중",
    },
    Translation {
        key: "status.saved",
        en: "Saved",
        ko: "저장됨",
    },
    Translation {
        key: "status.imported",
        en: "Imported",
        ko: "가져옴",
    },
    Translation {
        key: "status.autosaved",
        en: "Autosaved",
        ko: "자동 저장됨",
    },
    Translation {
        key: "status.recovered_autosave",
        en: "Recovered autosave",
        ko: "자동 저장 복구됨",
    },
    Translation {
        key: "status.discarded_autosave",
        en: "Discarded autosave",
        ko: "자동 저장 삭제됨",
    },
    Translation {
        key: "status.autosave_skipped",
        en: "Autosave recovery skipped",
        ko: "자동 저장 복구 건너뜀",
    },
    Translation {
        key: "status.theme",
        en: "Theme",
        ko: "테마",
    },
    Translation {
        key: "tab.untitled",
        en: "Untitled",
        ko: "제목 없음",
    },
    Translation {
        key: "tab.new",
        en: "New Tab",
        ko: "새 탭",
    },
    Translation {
        key: "tab.close",
        en: "Close tab",
        ko: "탭 닫기",
    },
    Translation {
        key: "tab.unsaved_marker",
        en: "unsaved",
        ko: "저장 안 됨",
    },
    Translation {
        key: "toolbar.select",
        en: "Select",
        ko: "선택",
    },
    Translation {
        key: "toolbar.move",
        en: "Move",
        ko: "이동",
    },
    Translation {
        key: "toolbar.rotate",
        en: "Rotate",
        ko: "회전",
    },
    Translation {
        key: "toolbar.scale",
        en: "Scale",
        ko: "스케일",
    },
    Translation {
        key: "toolbar.fit",
        en: "Fit",
        ko: "맞춤",
    },
    Translation {
        key: "toolbar.grid",
        en: "Grid",
        ko: "그리드",
    },
    Translation {
        key: "panel.model",
        en: "Model",
        ko: "모델",
    },
    Translation {
        key: "panel.inspector",
        en: "Inspector",
        ko: "검사기",
    },
    Translation {
        key: "panel.properties",
        en: "Properties",
        ko: "속성",
    },
    Translation {
        key: "panel.tasks",
        en: "Tasks",
        ko: "작업",
    },
    Translation {
        key: "panel.report",
        en: "Report",
        ko: "보고서",
    },
    Translation {
        key: "panel.console",
        en: "Console",
        ko: "콘솔",
    },
    Translation {
        key: "selection.solid",
        en: "Solid",
        ko: "솔리드",
    },
    Translation {
        key: "selection.face",
        en: "Face",
        ko: "면",
    },
    Translation {
        key: "selection.edge",
        en: "Edge",
        ko: "모서리",
    },
    Translation {
        key: "selection.vertex",
        en: "Vertex",
        ko: "꼭짓점",
    },
    Translation {
        key: "dialog.common.ok",
        en: "OK",
        ko: "확인",
    },
    Translation {
        key: "dialog.common.cancel",
        en: "Cancel",
        ko: "취소",
    },
    Translation {
        key: "dialog.common.apply",
        en: "Apply",
        ko: "적용",
    },
    Translation {
        key: "dialog.common.close",
        en: "Close",
        ko: "닫기",
    },
    Translation {
        key: "dialog.common.reset",
        en: "Reset",
        ko: "초기화",
    },
    Translation {
        key: "dialog.common.name",
        en: "Name",
        ko: "이름",
    },
    Translation {
        key: "dialog.common.width",
        en: "Width",
        ko: "너비",
    },
    Translation {
        key: "dialog.common.height",
        en: "Height",
        ko: "높이",
    },
    Translation {
        key: "dialog.common.depth",
        en: "Depth",
        ko: "깊이",
    },
    Translation {
        key: "dialog.common.radius",
        en: "Radius",
        ko: "반지름",
    },
    Translation {
        key: "dialog.common.angle",
        en: "Angle",
        ko: "각도",
    },
    Translation {
        key: "dialog.common.count",
        en: "Count",
        ko: "개수",
    },
    Translation {
        key: "dialog.common.spacing",
        en: "Spacing",
        ko: "간격",
    },
    Translation {
        key: "dialog.common.axis",
        en: "Axis",
        ko: "축",
    },
    Translation {
        key: "dialog.common.path",
        en: "Path",
        ko: "경로",
    },
    Translation {
        key: "dialog.common.size",
        en: "Size",
        ko: "크기",
    },
    Translation {
        key: "dialog.common.modified",
        en: "Modified",
        ko: "수정됨",
    },
    Translation {
        key: "dialog.common.file_size",
        en: "File size",
        ko: "파일 크기",
    },
    Translation {
        key: "dialog.common.error",
        en: "Error",
        ko: "오류",
    },
    Translation {
        key: "dialog.common.warning",
        en: "Warning",
        ko: "경고",
    },
    Translation {
        key: "dialog.common.info",
        en: "Info",
        ko: "정보",
    },
    Translation {
        key: "dialog.create_box.title",
        en: "Create Box",
        ko: "박스 생성",
    },
    Translation {
        key: "dialog.create_cylinder.title",
        en: "Create Cylinder",
        ko: "원통 생성",
    },
    Translation {
        key: "dialog.create_sphere.title",
        en: "Create Sphere",
        ko: "구 생성",
    },
    Translation {
        key: "dialog.create_cone.title",
        en: "Create Cone",
        ko: "원뿔 생성",
    },
    Translation {
        key: "dialog.create_torus.title",
        en: "Create Torus",
        ko: "토러스 생성",
    },
    Translation {
        key: "dialog.create_tube.title",
        en: "Create Tube",
        ko: "튜브 생성",
    },
    Translation {
        key: "dialog.create_prism.title",
        en: "Create Prism",
        ko: "프리즘 생성",
    },
    Translation {
        key: "dialog.create_wedge.title",
        en: "Create Wedge",
        ko: "쐐기 생성",
    },
    Translation {
        key: "dialog.create_ellipsoid.title",
        en: "Create Ellipsoid",
        ko: "타원체 생성",
    },
    Translation {
        key: "dialog.create_helix.title",
        en: "Create Helix",
        ko: "나선 생성",
    },
    Translation {
        key: "help.shortcuts",
        en: "Keyboard Shortcuts",
        ko: "키보드 단축키",
    },
    Translation {
        key: "help.about",
        en: "About CADKernel",
        ko: "CADKernel 정보",
    },
    Translation {
        key: "tooltip.new_tab",
        en: "Create a new document tab",
        ko: "새 문서 탭 만들기",
    },
    Translation {
        key: "tooltip.close_tab",
        en: "Close the active document tab",
        ko: "활성 문서 탭 닫기",
    },
    Translation {
        key: "tooltip.drag_drop",
        en: "Drop CAD files to open them in a new tab",
        ko: "CAD 파일을 놓으면 새 탭에서 엽니다",
    },
    Translation {
        key: "access.menu_bar",
        en: "Menu bar",
        ko: "메뉴 막대",
    },
    Translation {
        key: "access.document_tabs",
        en: "Document tabs",
        ko: "문서 탭",
    },
    Translation {
        key: "access.viewport",
        en: "3D viewport",
        ko: "3D 뷰포트",
    },
    Translation {
        key: "access.model_tree",
        en: "Model tree",
        ko: "모델 트리",
    },
    Translation {
        key: "access.inspector",
        en: "Inspector",
        ko: "검사기",
    },
    Translation {
        key: "access.status_bar",
        en: "Status bar",
        ko: "상태 표시줄",
    },
    Translation {
        key: "access.report_panel",
        en: "Report panel",
        ko: "보고서 패널",
    },
    Translation {
        key: "access.toolbar",
        en: "Toolbar",
        ko: "도구 모음",
    },
    Translation {
        key: "access.activity_rail",
        en: "Activity rail",
        ko: "활동 레일",
    },
    Translation {
        key: "access.command_palette",
        en: "Command palette",
        ko: "명령 팔레트",
    },
    Translation {
        key: "fallback.missing",
        en: "Missing translation",
        ko: "번역 없음",
    },
];

#[cfg(test)]
pub fn t(key: &str) -> &'static str {
    translate(Language::En, key)
}

pub fn translate(language: Language, key: &str) -> &'static str {
    TRANSLATIONS
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| match language {
            Language::En => entry.en,
            Language::Ko => entry.ko,
        })
        .unwrap_or("<?>")
}

#[cfg(test)]
pub fn keys() -> impl Iterator<Item = &'static str> {
    TRANSLATIONS.iter().map(|entry| entry.key)
}

#[cfg(test)]
pub fn missing_for(language: Language) -> Vec<&'static str> {
    TRANSLATIONS
        .iter()
        .filter_map(|entry| {
            let value = match language {
                Language::En => entry.en,
                Language::Ko => entry.ko,
            };
            value.trim().is_empty().then_some(entry.key)
        })
        .collect()
}

#[cfg(test)]
pub fn has_key(key: &str) -> bool {
    TRANSLATIONS.iter().any(|entry| entry.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_helpers_cover_the_table() {
        assert_eq!(t("menu.file"), "File");
        assert!(keys().count() >= 200);
        assert!(missing_for(Language::En).is_empty());
        assert!(missing_for(Language::Ko).is_empty());
        assert!(has_key("settings.theme.system"));
    }
}
