use std::collections::HashSet;

use ratatui::widgets::TableState;
use sysinfo::{Pid, System};
use ttop::{App, ProcessGroup, ProcessInfo, SortMode, format_memory, format_bytes};

// ============================================================
// Test Helpers
// ============================================================

/// Create a test App with mock process groups (bypasses real system info)
fn create_test_app_with_groups(groups: Vec<ProcessGroup>) -> App {
    let mut app = App {
        system: System::new(),
        table_state: TableState::default(),
        groups,
        expanded_pids: HashSet::new(),
        sort_mode: SortMode::Cpu,
        sort_ascending: false,
        filter_text: String::new(),
        search_active: false,
    };
    app.table_state.select(Some(0));
    app
}

/// Create a mock ProcessGroup for testing
fn create_group(
    pid: u32,
    name: &str,
    cpu: f32,
    memory: u64,
    disk_read: u64,
    disk_write: u64,
    children: Vec<ProcessInfo>,
) -> ProcessGroup {
    ProcessGroup {
        pid: Pid::from_u32(pid),
        name: name.to_string(),
        cpu_usage: cpu,
        memory,
        disk_read,
        disk_write,
        children,
        expanded: false,
    }
}

/// Create a mock ProcessInfo (child process) for testing
fn create_child(
    pid: u32,
    name: &str,
    cpu: f32,
    memory: u64,
    disk_read: u64,
    disk_write: u64,
) -> ProcessInfo {
    ProcessInfo {
        pid: Pid::from_u32(pid),
        name: name.to_string(),
        cpu_usage: cpu,
        memory,
        disk_read,
        disk_write,
    }
}

/// Create a standard set of test groups for sorting tests
fn create_test_groups() -> Vec<ProcessGroup> {
    vec![
        create_group(100, "Safari", 25.0, 500 * 1024 * 1024, 1000, 500, vec![
            create_child(101, "WebKit Networking", 5.0, 100 * 1024 * 1024, 200, 100),
            create_child(102, "WebKit GPU", 10.0, 200 * 1024 * 1024, 300, 200),
        ]),
        create_group(200, "Chrome", 45.0, 1024 * 1024 * 1024, 5000, 2000, vec![
            create_child(201, "Chrome Helper", 15.0, 300 * 1024 * 1024, 1000, 500),
        ]),
        create_group(300, "Terminal", 5.0, 50 * 1024 * 1024, 100, 50, vec![]),
        create_group(400, "Finder", 2.0, 150 * 1024 * 1024, 2000, 1000, vec![]),
        create_group(500, "Activity Monitor", 10.0, 80 * 1024 * 1024, 500, 250, vec![]),
    ]
}

// ============================================================
// SortMode Tests
// ============================================================

#[test]
fn test_sort_mode_label() {
    assert_eq!(SortMode::Cpu.label(), "CPU");
    assert_eq!(SortMode::Memory.label(), "MEM");
    assert_eq!(SortMode::DiskIo.label(), "DISK");
    assert_eq!(SortMode::Name.label(), "NAME");
}

#[test]
fn test_sort_by_cpu_descending() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.sort_mode = SortMode::Cpu;
    app.sort_ascending = false;
    app.sort_groups(&mut app.groups.clone());

    let mut groups = app.groups.clone();
    app.sort_groups(&mut groups);

    assert_eq!(groups[0].name, "Chrome");
    assert_eq!(groups[1].name, "Safari");
    assert_eq!(groups[4].name, "Finder");
}

#[test]
fn test_sort_by_cpu_ascending() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.sort_mode = SortMode::Cpu;
    app.sort_ascending = true;

    let mut groups = app.groups.clone();
    app.sort_groups(&mut groups);

    assert_eq!(groups[0].name, "Finder");
    assert_eq!(groups[4].name, "Chrome");
}

#[test]
fn test_sort_by_memory_descending() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.sort_mode = SortMode::Memory;
    app.sort_ascending = false;

    let mut groups = app.groups.clone();
    app.sort_groups(&mut groups);

    assert_eq!(groups[0].name, "Chrome");
    assert_eq!(groups[4].name, "Terminal");
}

#[test]
fn test_sort_by_memory_ascending() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.sort_mode = SortMode::Memory;
    app.sort_ascending = true;

    let mut groups = app.groups.clone();
    app.sort_groups(&mut groups);

    assert_eq!(groups[0].name, "Terminal");
    assert_eq!(groups[4].name, "Chrome");
}

#[test]
fn test_sort_by_disk_descending() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.sort_mode = SortMode::DiskIo;
    app.sort_ascending = false;

    let mut groups = app.groups.clone();
    app.sort_groups(&mut groups);

    assert_eq!(groups[0].name, "Chrome");
    assert_eq!(groups[4].name, "Terminal");
}

#[test]
fn test_sort_by_disk_ascending() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.sort_mode = SortMode::DiskIo;
    app.sort_ascending = true;

    let mut groups = app.groups.clone();
    app.sort_groups(&mut groups);

    assert_eq!(groups[0].name, "Terminal");
    assert_eq!(groups[4].name, "Chrome");
}

#[test]
fn test_sort_by_name_descending() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.sort_mode = SortMode::Name;
    app.sort_ascending = false;

    let mut groups = app.groups.clone();
    app.sort_groups(&mut groups);

    assert_eq!(groups[0].name, "Terminal");
    assert_eq!(groups[4].name, "Activity Monitor");
}

#[test]
fn test_sort_by_name_ascending() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.sort_mode = SortMode::Name;
    app.sort_ascending = true;

    let mut groups = app.groups.clone();
    app.sort_groups(&mut groups);

    assert_eq!(groups[0].name, "Activity Monitor");
    assert_eq!(groups[4].name, "Terminal");
}

#[test]
fn test_toggle_sort_mode_changes_mode() {
    let mut app = create_test_app_with_groups(create_test_groups());
    assert_eq!(app.sort_mode, SortMode::Cpu);

    app.toggle_sort_mode(SortMode::Memory);
    assert_eq!(app.sort_mode, SortMode::Memory);
    assert!(!app.sort_ascending);

    app.toggle_sort_mode(SortMode::DiskIo);
    assert_eq!(app.sort_mode, SortMode::DiskIo);
    assert!(!app.sort_ascending);

    app.toggle_sort_mode(SortMode::Name);
    assert_eq!(app.sort_mode, SortMode::Name);
    assert!(!app.sort_ascending);
}

#[test]
fn test_toggle_sort_mode_toggles_direction() {
    let mut app = create_test_app_with_groups(create_test_groups());

    assert_eq!(app.sort_mode, SortMode::Cpu);
    assert!(!app.sort_ascending);

    app.toggle_sort_mode(SortMode::Cpu);
    assert_eq!(app.sort_mode, SortMode::Cpu);
    assert!(app.sort_ascending);

    app.toggle_sort_mode(SortMode::Cpu);
    assert_eq!(app.sort_mode, SortMode::Cpu);
    assert!(!app.sort_ascending);
}

#[test]
fn test_sort_children_by_cpu() {
    let groups = vec![
        create_group(100, "Safari", 25.0, 500 * 1024 * 1024, 1000, 500, vec![
            create_child(101, "WebKit A", 5.0, 100 * 1024 * 1024, 200, 100),
            create_child(102, "WebKit B", 15.0, 200 * 1024 * 1024, 300, 200),
            create_child(103, "WebKit C", 10.0, 150 * 1024 * 1024, 250, 150),
        ]),
    ];

    let mut app = create_test_app_with_groups(groups);
    app.sort_mode = SortMode::Cpu;
    app.sort_ascending = false;

    let mut groups = app.groups.clone();
    app.sort_groups(&mut groups);

    assert_eq!(groups[0].children[0].name, "WebKit B");
    assert_eq!(groups[0].children[1].name, "WebKit C");
    assert_eq!(groups[0].children[2].name, "WebKit A");
}

// ============================================================
// Filter/Search Tests
// ============================================================

#[test]
fn test_filter_empty_shows_all() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.filter_text = String::new();

    let rows = app.get_visible_rows();
    let group_count = rows.iter().filter(|r| r.is_group).count();

    assert_eq!(group_count, 5);
}

#[test]
fn test_filter_by_exact_name() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.filter_text = "Safari".to_string();

    let rows = app.get_visible_rows();
    let group_count = rows.iter().filter(|r| r.is_group).count();

    assert_eq!(group_count, 1);
    assert_eq!(rows[0].name, "Safari");
}

#[test]
fn test_filter_case_insensitive() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.filter_text = "safari".to_string();

    let rows = app.get_visible_rows();
    let group_count = rows.iter().filter(|r| r.is_group).count();

    assert_eq!(group_count, 1);
    assert_eq!(rows[0].name, "Safari");
}

#[test]
fn test_filter_partial_match() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.filter_text = "term".to_string();

    let rows = app.get_visible_rows();
    let group_count = rows.iter().filter(|r| r.is_group).count();

    assert_eq!(group_count, 1);
    assert_eq!(rows[0].name, "Terminal");
}

#[test]
fn test_filter_no_match() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.filter_text = "nonexistent".to_string();

    let rows = app.get_visible_rows();

    assert!(rows.is_empty());
}

#[test]
fn test_filter_matches_child_shows_parent() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.filter_text = "WebKit".to_string();
    app.expanded_pids.insert(Pid::from_u32(100));
    app.groups[0].expanded = true;

    let rows = app.get_visible_rows();

    assert!(rows.iter().any(|r| r.name == "Safari"));
    let webkit_rows: Vec<_> = rows.iter().filter(|r| r.name.contains("WebKit")).collect();
    assert_eq!(webkit_rows.len(), 2);
}

#[test]
fn test_filter_multiple_matches() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.filter_text = "er".to_string();

    let rows = app.get_visible_rows();
    let group_names: Vec<_> = rows.iter().filter(|r| r.is_group).map(|r| &r.name).collect();

    assert!(group_names.contains(&&"Finder".to_string()));
    assert!(group_names.contains(&&"Terminal".to_string()));
}

#[test]
fn test_clear_filter() {
    let mut app = create_test_app_with_groups(create_test_groups());

    app.filter_text = "Safari".to_string();
    let rows = app.get_visible_rows();
    assert_eq!(rows.iter().filter(|r| r.is_group).count(), 1);

    app.filter_text.clear();
    let rows = app.get_visible_rows();
    assert_eq!(rows.iter().filter(|r| r.is_group).count(), 5);
}

// ============================================================
// Navigation Tests
// ============================================================

#[test]
fn test_navigation_next() {
    let mut app = create_test_app_with_groups(create_test_groups());
    assert_eq!(app.table_state.selected(), Some(0));

    app.next();
    assert_eq!(app.table_state.selected(), Some(1));

    app.next();
    assert_eq!(app.table_state.selected(), Some(2));
}

#[test]
fn test_navigation_previous() {
    let mut app = create_test_app_with_groups(create_test_groups());
    app.table_state.select(Some(3));

    app.previous();
    assert_eq!(app.table_state.selected(), Some(2));

    app.previous();
    assert_eq!(app.table_state.selected(), Some(1));
}

#[test]
fn test_navigation_stays_at_top() {
    let mut app = create_test_app_with_groups(create_test_groups());
    assert_eq!(app.table_state.selected(), Some(0));

    app.previous();
    assert_eq!(app.table_state.selected(), Some(0));
}

#[test]
fn test_navigation_stays_at_bottom() {
    let mut app = create_test_app_with_groups(create_test_groups());
    let row_count = app.visible_row_count();
    app.table_state.select(Some(row_count - 1));

    app.next();
    assert_eq!(app.table_state.selected(), Some(row_count - 1));
}

#[test]
fn test_navigation_empty_groups() {
    let mut app = create_test_app_with_groups(vec![]);

    app.next();
    app.previous();
}

// ============================================================
// Toggle Expand/Collapse Tests
// ============================================================

#[test]
fn test_toggle_expand_group() {
    let mut app = create_test_app_with_groups(create_test_groups());
    let safari_pid = Pid::from_u32(100);

    assert!(!app.expanded_pids.contains(&safari_pid));

    let safari_idx = app.groups.iter().position(|g| g.name == "Safari").unwrap();
    app.table_state.select(Some(safari_idx));

    app.toggle_current();

    assert!(app.expanded_pids.contains(&safari_pid));
}

#[test]
fn test_toggle_collapse_group() {
    let mut app = create_test_app_with_groups(create_test_groups());
    let safari_pid = Pid::from_u32(100);

    app.expanded_pids.insert(safari_pid);
    app.groups[0].expanded = true;

    let safari_idx = app.groups.iter().position(|g| g.name == "Safari").unwrap();
    app.table_state.select(Some(safari_idx));

    app.toggle_current();

    assert!(!app.expanded_pids.contains(&safari_pid));
}

#[test]
fn test_toggle_group_without_children_no_op() {
    let mut app = create_test_app_with_groups(create_test_groups());

    let terminal_idx = app.groups.iter().position(|g| g.name == "Terminal").unwrap();
    app.table_state.select(Some(terminal_idx));

    let expanded_before = app.expanded_pids.len();
    app.toggle_current();
    let expanded_after = app.expanded_pids.len();

    assert_eq!(expanded_before, expanded_after);
}

#[test]
fn test_toggle_child_process_no_op() {
    let mut app = create_test_app_with_groups(create_test_groups());

    let safari_idx = app.groups.iter().position(|g| g.name == "Safari").unwrap();
    app.expanded_pids.insert(Pid::from_u32(100));
    app.groups[safari_idx].expanded = true;

    let rows = app.get_visible_rows();
    let child_idx = rows.iter().position(|r| !r.is_group).unwrap();

    app.table_state.select(Some(child_idx));

    let expanded_before = app.expanded_pids.clone();
    app.toggle_current();
    let expanded_after = app.expanded_pids.clone();

    assert_eq!(expanded_before, expanded_after);
}

#[test]
fn test_toggle_expand_collapse_cycle() {
    let mut app = create_test_app_with_groups(create_test_groups());
    let safari_pid = Pid::from_u32(100);

    let safari_idx = app.groups.iter().position(|g| g.name == "Safari").unwrap();
    app.table_state.select(Some(safari_idx));

    assert!(!app.expanded_pids.contains(&safari_pid));
    assert!(!app.groups[safari_idx].expanded);
    let initial_row_count = app.visible_row_count();

    app.expanded_pids.insert(safari_pid);
    app.groups[safari_idx].expanded = true;

    assert!(app.expanded_pids.contains(&safari_pid));
    let expanded_row_count = app.visible_row_count();
    assert!(expanded_row_count > initial_row_count);
    assert_eq!(expanded_row_count, initial_row_count + 2);

    app.expanded_pids.remove(&safari_pid);
    app.groups[safari_idx].expanded = false;

    assert!(!app.expanded_pids.contains(&safari_pid));
    let collapsed_row_count = app.visible_row_count();
    assert_eq!(collapsed_row_count, initial_row_count);
}

#[test]
fn test_toggle_multiple_groups() {
    let mut app = create_test_app_with_groups(create_test_groups());

    let safari_idx = app.groups.iter().position(|g| g.name == "Safari").unwrap();
    let chrome_idx = app.groups.iter().position(|g| g.name == "Chrome").unwrap();

    let initial_count = app.visible_row_count();

    app.expanded_pids.insert(Pid::from_u32(100));
    app.groups[safari_idx].expanded = true;
    let after_safari = app.visible_row_count();
    assert_eq!(after_safari, initial_count + 2);

    app.expanded_pids.insert(Pid::from_u32(200));
    app.groups[chrome_idx].expanded = true;
    let after_chrome = app.visible_row_count();
    assert_eq!(after_chrome, initial_count + 3);

    assert!(app.expanded_pids.contains(&Pid::from_u32(100)));
    assert!(app.expanded_pids.contains(&Pid::from_u32(200)));
}

#[test]
fn test_expanded_group_shows_children() {
    let mut app = create_test_app_with_groups(create_test_groups());

    let rows_before = app.get_visible_rows();
    let count_before = rows_before.len();

    let safari_idx = app.groups.iter().position(|g| g.name == "Safari").unwrap();
    app.expanded_pids.insert(Pid::from_u32(100));
    app.groups[safari_idx].expanded = true;

    let rows_after = app.get_visible_rows();
    let count_after = rows_after.len();

    assert_eq!(count_after, count_before + 2);
}

// ============================================================
// Utility Function Tests
// ============================================================

#[test]
fn test_format_memory_bytes() {
    assert_eq!(format_memory(500), "500 B");
    assert_eq!(format_memory(0), "0 B");
}

#[test]
fn test_format_memory_kilobytes() {
    assert_eq!(format_memory(1024), "1.0 KB");
    assert_eq!(format_memory(1536), "1.5 KB");
    assert_eq!(format_memory(10 * 1024), "10.0 KB");
}

#[test]
fn test_format_memory_megabytes() {
    assert_eq!(format_memory(1024 * 1024), "1.0 MB");
    assert_eq!(format_memory(512 * 1024 * 1024), "512.0 MB");
}

#[test]
fn test_format_memory_gigabytes() {
    assert_eq!(format_memory(1024 * 1024 * 1024), "1.0 GB");
    assert_eq!(format_memory(2 * 1024 * 1024 * 1024), "2.0 GB");
}

#[test]
fn test_format_bytes_all_units() {
    assert_eq!(format_bytes(500), "500 B");
    assert_eq!(format_bytes(1024), "1.0 KB");
    assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
    assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
}

// ============================================================
// Integration-style Tests
// ============================================================

#[test]
fn test_sort_then_filter() {
    let mut app = create_test_app_with_groups(create_test_groups());

    app.sort_mode = SortMode::Name;
    app.sort_ascending = true;
    let mut groups = app.groups.clone();
    app.sort_groups(&mut groups);
    app.groups = groups;

    app.filter_text = "a".to_string();

    let rows = app.get_visible_rows();
    let group_names: Vec<_> = rows.iter().filter(|r| r.is_group).map(|r| &r.name).collect();

    assert!(group_names.len() >= 2);
}

#[test]
fn test_filter_then_navigate() {
    let mut app = create_test_app_with_groups(create_test_groups());

    app.filter_text = "Chrome".to_string();
    app.table_state.select(Some(0));

    let rows = app.get_visible_rows();
    assert_eq!(rows.iter().filter(|r| r.is_group).count(), 1);

    app.next();
    assert_eq!(app.table_state.selected(), Some(0));
}

#[test]
fn test_visible_row_count() {
    let mut app = create_test_app_with_groups(create_test_groups());

    assert_eq!(app.visible_row_count(), 5);

    app.expanded_pids.insert(Pid::from_u32(100));
    let safari_idx = app.groups.iter().position(|g| g.name == "Safari").unwrap();
    app.groups[safari_idx].expanded = true;
    assert_eq!(app.visible_row_count(), 7);

    app.expanded_pids.insert(Pid::from_u32(200));
    let chrome_idx = app.groups.iter().position(|g| g.name == "Chrome").unwrap();
    app.groups[chrome_idx].expanded = true;
    assert_eq!(app.visible_row_count(), 8);
}

#[test]
fn test_display_row_properties() {
    let mut app = create_test_app_with_groups(create_test_groups());

    app.expanded_pids.insert(Pid::from_u32(100));
    let safari_idx = app.groups.iter().position(|g| g.name == "Safari").unwrap();
    app.groups[safari_idx].expanded = true;

    let rows = app.get_visible_rows();

    let safari_row = rows.iter().find(|r| r.name == "Safari").unwrap();
    assert!(safari_row.is_group);
    assert!(safari_row.expanded);
    assert!(safari_row.has_children);

    let child_row = rows.iter().find(|r| r.name.contains("WebKit")).unwrap();
    assert!(!child_row.is_group);
    assert!(!child_row.expanded);
    assert!(!child_row.has_children);
}
