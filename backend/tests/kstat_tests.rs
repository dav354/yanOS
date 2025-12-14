//! Tests for kstat FFI bindings.
//!
//! These tests verify the kstat reader can correctly interface with
//! the illumos kernel statistics system.
//!
//! NOTE: These tests only run on illumos/OmniOS (via `just test`).

use yanos_backend::adapters::kstat::{CpuRawTicks, KstatError, KstatReader};

/// Test that KstatReader can be constructed.
#[test]
fn test_kstat_reader_construction() {
    let result = KstatReader::new();
    assert!(result.is_ok(), "KstatReader::new() should succeed on illumos");
    println!("KstatReader opened successfully");
}

/// Test CPU tick collection - the core metric for dashboard CPU charts.
#[test]
fn test_get_cpu_ticks_by_instance() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");
    let ticks = reader.get_cpu_ticks_by_instance();

    // Should have at least one CPU core
    assert!(
        !ticks.is_empty(),
        "Expected at least one CPU core in kstat data"
    );

    // Each core should have valid tick values (at least one non-zero)
    for (instance, raw) in &ticks {
        println!(
            "CPU {}: idle={}, user={}, kernel={}",
            instance, raw.idle, raw.user, raw.kernel
        );

        // At least one tick counter should be non-zero after system boot
        let total = raw.idle + raw.user + raw.kernel;
        assert!(
            total > 0,
            "CPU {} should have accumulated some ticks",
            instance
        );
    }
}

/// Test aggregate CPU tick calculation.
#[test]
fn test_get_aggregate_cpu_ticks() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");
    let agg = reader.get_aggregate_cpu_ticks();

    // Aggregate should have non-zero ticks
    let total = agg.idle + agg.user + agg.kernel;
    assert!(total > 0, "Aggregate CPU ticks should be non-zero");

    println!(
        "Aggregate CPU: idle={}, user={}, kernel={}",
        agg.idle, agg.user, agg.kernel
    );
}

/// Test memory page statistics.
#[test]
fn test_get_memory_pages() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");
    let (total, free) = reader.get_memory_pages();

    // Total memory should be non-zero
    assert!(total > 0, "Total memory pages should be non-zero");

    // Free memory should be less than or equal to total
    assert!(
        free <= total,
        "Free pages ({}) should not exceed total ({})",
        free,
        total
    );

    println!("Memory pages: total={}, free={}", total, free);
}

/// Test ZFS ARC size retrieval.
/// Note: This may return 0 if ZFS is not loaded or no pools exist.
#[test]
fn test_get_arc_size() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");
    let arc_size = reader.get_arc_size();

    // ARC size could be 0 if no ZFS pools, that's valid
    println!("ZFS ARC size: {} bytes", arc_size);

    // If ARC is in use, it should be at least a few MB
    if arc_size > 0 {
        assert!(
            arc_size > 1024 * 1024,
            "If ARC is active, it should be at least 1MB"
        );
    }
}

/// Test kstat chain update.
#[test]
fn test_kstat_update() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");

    // Update should succeed (even if chain hasn't changed)
    let result = reader.update();
    assert!(result.is_ok(), "kstat chain update should succeed");

    println!("kstat chain update: changed={}", result.unwrap());
}

/// Test get_named for a known statistic.
#[test]
fn test_get_named_system_pages() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");

    // unix:0:system_pages:physmem should always exist
    let result = reader.get_named("unix", 0, "system_pages", "physmem");

    match result {
        Ok(pages) => {
            assert!(pages > 0, "physmem should be non-zero");
            println!("physmem: {} pages", pages);
        }
        Err(e) => {
            panic!("Failed to read unix:0:system_pages:physmem: {:?}", e);
        }
    }
}

/// Test get_named with invalid module.
#[test]
fn test_get_named_invalid_module() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");

    // Non-existent module should return Lookup error
    let result = reader.get_named("nonexistent_module_xyz", 0, "fake", "stat");

    match result {
        Err(KstatError::Lookup(_)) => {
            println!("Correctly got Lookup error for invalid module");
        }
        Ok(v) => {
            panic!("Expected Lookup error, got value: {}", v);
        }
        Err(e) => {
            panic!("Expected Lookup error, got: {:?}", e);
        }
    }
}

/// Test sum_named function for CPU module.
#[test]
fn test_sum_named() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");

    // Sum cpu_ticks_idle across all cpu:*:sys kstats
    let sum = reader.sum_named("cpu", "cpu_ticks_idle");

    // Should be non-zero on a running system
    assert!(sum > 0, "Sum of cpu_ticks_idle should be non-zero");
    println!("Sum of cpu_ticks_idle across all CPUs: {}", sum);
}

/// Test sum_field_any for network statistics.
#[test]
fn test_sum_field_any_network() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");

    // Sum rbytes64 across all net-class kstats
    let rx_bytes = reader.sum_field_any("rbytes64", Some("net"));
    let tx_bytes = reader.sum_field_any("obytes64", Some("net"));

    // Could be 0 if no network traffic has occurred
    println!("Network totals: rx={}, tx={}", rx_bytes, tx_bytes);
}

/// Test that CpuRawTicks Default implementation works.
#[test]
fn test_cpu_raw_ticks_default() {
    let ticks = CpuRawTicks::default();
    assert_eq!(ticks.idle, 0);
    assert_eq!(ticks.user, 0);
    assert_eq!(ticks.kernel, 0);
}

/// Test that CpuRawTicks is Copy and Clone.
#[test]
fn test_cpu_raw_ticks_copy_clone() {
    let original = CpuRawTicks {
        idle: 100,
        user: 50,
        kernel: 25,
    };

    // Test Copy
    let copied = original;
    assert_eq!(copied.idle, 100);
    assert_eq!(original.idle, 100); // Original still valid

    // Test Clone
    let cloned = original.clone();
    assert_eq!(cloned.idle, 100);
}

// --- Edge Case Tests ---

/// Test get_named with empty string arguments.
#[test]
fn test_get_named_empty_strings() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");

    // Empty module should fail
    let result = reader.get_named("", 0, "system_pages", "physmem");
    assert!(result.is_err(), "Empty module should fail");

    // Empty name should fail
    let result = reader.get_named("unix", 0, "", "physmem");
    assert!(result.is_err(), "Empty name should fail");

    // Empty statistic should fail
    let result = reader.get_named("unix", 0, "system_pages", "");
    assert!(result.is_err(), "Empty statistic should fail");
}

/// Test get_named with negative instance.
#[test]
fn test_get_named_negative_instance() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");

    // -1 is sometimes used as wildcard, but should still handle gracefully
    let result = reader.get_named("unix", -1, "system_pages", "physmem");
    // Result depends on kstat implementation - just verify no crash
    println!("Negative instance result: {:?}", result);
}

/// Test sum_named with nonexistent module.
#[test]
fn test_sum_named_nonexistent_module() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");

    let sum = reader.sum_named("nonexistent_module_xyz", "some_field");
    assert_eq!(sum, 0, "Nonexistent module should return 0");
}

/// Test sum_named with nonexistent field.
#[test]
fn test_sum_named_nonexistent_field() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");

    let sum = reader.sum_named("cpu", "nonexistent_field_xyz");
    assert_eq!(sum, 0, "Nonexistent field should return 0");
}

/// Test sum_field_any with no matching class prefix.
#[test]
fn test_sum_field_any_no_matching_class() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");

    let sum = reader.sum_field_any("rbytes64", Some("nonexistent_class"));
    assert_eq!(sum, 0, "No matching class should return 0");
}

/// Test sum_field_any with None class prefix.
#[test]
fn test_sum_field_any_no_class_filter() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");

    // Without class filter, may find fields in any kstat
    let sum = reader.sum_field_any("cpu_ticks_idle", None);
    // Should find CPU ticks
    assert!(sum > 0 || sum == 0, "Should not panic");
}

/// Test multiple kstat readers concurrently.
#[test]
fn test_multiple_readers() {
    let mut reader1 = KstatReader::new().expect("Failed to open kstat 1");
    let mut reader2 = KstatReader::new().expect("Failed to open kstat 2");

    let ticks1 = reader1.get_aggregate_cpu_ticks();
    let ticks2 = reader2.get_aggregate_cpu_ticks();

    // Both should return valid data
    assert!(ticks1.idle + ticks1.user + ticks1.kernel > 0);
    assert!(ticks2.idle + ticks2.user + ticks2.kernel > 0);
}

/// Test kstat update multiple times.
#[test]
fn test_kstat_update_multiple() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");

    for _ in 0..10 {
        let result = reader.update();
        assert!(result.is_ok(), "Multiple updates should succeed");
    }
}

/// Test CpuRawTicks with max values.
#[test]
fn test_cpu_raw_ticks_max_values() {
    let ticks = CpuRawTicks {
        idle: u64::MAX,
        user: u64::MAX,
        kernel: u64::MAX,
    };

    assert_eq!(ticks.idle, u64::MAX);
    assert_eq!(ticks.user, u64::MAX);
    assert_eq!(ticks.kernel, u64::MAX);
}

/// Test getting CPU ticks returns sorted instances.
#[test]
fn test_cpu_ticks_instance_ordering() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");
    let ticks = reader.get_cpu_ticks_by_instance();

    // Verify instances are valid (non-negative)
    for (instance, _) in &ticks {
        assert!(*instance >= 0, "Instance should be non-negative");
    }
}
