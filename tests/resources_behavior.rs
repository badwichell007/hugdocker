use dockerctl::resources::{
    cpu_percent, format_bytes, ResourcePanelData, ResourceRow, ResourceSummary,
};

#[test]
fn resource_math_formats_bytes_and_cpu_percent() {
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(1_536), "1.5 KiB");
    assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MiB");

    let percent = cpu_percent(150, 100, 1_000, 500, 2);
    assert!((percent - 20.0).abs() < f64::EPSILON);

    assert_eq!(cpu_percent(100, 100, 1_000, 500, 2), 0.0);
    assert_eq!(cpu_percent(150, 100, 1_000, 1_000, 2), 0.0);
}

#[test]
fn resource_summary_aggregates_success_rows_and_errors() {
    let rows = vec![
        ResourceRow::ok("web", "web_1", "UP", 12.5, 512, 1_024, 100, 200, 300, 400),
        ResourceRow::ok(
            "worker", "worker_1", "UNHL", 7.5, 256, 1_024, 10, 20, 30, 40,
        ),
        ResourceRow::error("gone", "gone_1", "DOWN", "Docker returned 404"),
    ];

    let summary = ResourceSummary::from_rows(&rows);

    assert_eq!(summary.containers, 3);
    assert_eq!(summary.error_count, 1);
    assert!((summary.cpu_percent - 20.0).abs() < f64::EPSILON);
    assert_eq!(summary.memory_usage_bytes, 768);
    assert_eq!(summary.memory_limit_bytes, 2_048);
    assert!((summary.memory_percent - 37.5).abs() < f64::EPSILON);
    assert_eq!(summary.network_rx_bytes, 110);
    assert_eq!(summary.network_tx_bytes, 220);
    assert_eq!(summary.block_read_bytes, 330);
    assert_eq!(summary.block_write_bytes, 440);
}

#[test]
fn resource_panel_data_marks_loading_empty_and_stale_states() {
    let loading = ResourcePanelData::loading("mingli");
    assert!(loading.loading);
    assert_eq!(loading.project, "mingli");
    assert!(loading.rows.is_empty());

    let empty = ResourcePanelData::sampled("mingli", 1_000, Vec::new());
    assert!(!empty.loading);
    assert!(empty.rows.is_empty());
    assert_eq!(empty.summary.containers, 0);

    let stale = empty.with_stale(true);
    assert!(stale.stale);
}
