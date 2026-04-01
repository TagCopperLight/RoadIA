use crate::map::road::LinkType;
use crate::simulation::kinematics::{approach_speed, arrival_time, leave_time, v_stop_at};

// ---- arrival_time ----

#[test]
fn arrival_time_zero_distance() {
    assert_eq!(arrival_time(0.0, 10.0, 5.0, 4.0, 3.0), 0.0);
}

#[test]
fn arrival_time_same_speed_cruise() {
    // v0 == v1: no acceleration needed, pure cruise
    let t = arrival_time(100.0, 10.0, 10.0, 4.0, 3.0);
    assert!((t - 10.0).abs() < 1e-3, "expected 10.0, got {t}");
}

#[test]
fn arrival_time_accel_with_cruise() {
    // v0=0, v1=10: accel phase then cruise
    // t_accel = 10/4 = 2.5s, d_accel = 2.5*(0+10)/2 = 12.5m
    // remaining = 50 - 12.5 = 37.5m at 10m/s = 3.75s
    // total = 6.25s
    let t = arrival_time(50.0, 0.0, 10.0, 4.0, 3.0);
    assert!((t - 6.25).abs() < 1e-3, "expected 6.25, got {t}");
}

#[test]
fn arrival_time_accel_quadratic_fallback() {
    // dist=1.0 < d_accel=12.5 → quadratic branch
    // t = (-0 + sqrt(0 + 2*4*1)) / 4 = sqrt(8)/4 ≈ 0.7071
    let t = arrival_time(1.0, 0.0, 10.0, 4.0, 3.0);
    assert!((t - (8.0f32).sqrt() / 4.0).abs() < 1e-3, "got {t}");
}

#[test]
fn arrival_time_decel_with_cruise() {
    // v0=20, v1=5: t_decel=(20-5)/3=5s, d_decel=5*(20+5)/2=62.5m
    // dist=200 > d_decel → cruise at max(v0,v1)=20 for remaining
    // cruise_dist = 200-62.5 = 137.5, at v=20 → 6.875s
    // total = 5 + 6.875 = 11.875s
    let t = arrival_time(200.0, 20.0, 5.0, 4.0, 3.0);
    assert!((t - 11.875).abs() < 1e-3, "expected 11.875, got {t}");
}

#[test]
fn arrival_time_decel_quadratic_positive_discriminant() {
    // v0=10, v1=5, a=4, d=3, dist=10
    // d_decel = (10-5)/3 * (10+5)/2 = 5/3*7.5 = 12.5 > 10 → quadratic
    // discriminant = 100 - 2*3*10 = 40 > 0
    // t = (10 - sqrt(40)) / 3 ≈ (10 - 6.3246) / 3 ≈ 1.225
    let t = arrival_time(10.0, 10.0, 5.0, 4.0, 3.0);
    let expected = (10.0 - 40.0f32.sqrt()) / 3.0;
    assert!((t - expected).abs() < 1e-3, "expected {expected}, got {t}");
}

#[test]
fn arrival_time_decel_small_v0_cruise_branch() {
    // v0=0.1, v1=0.0, dist=1.0
    // d_decel = (0.1/3.0)*(0.1/2.0) = 0.00167m << dist → cruise branch (not quadratic)
    // t_decel = 0.1/3 ≈ 0.0333s, cruise_dist = 1.0-0.00167 ≈ 0.998 at 0.1 → 9.983s
    // total ≈ 10.017
    let t = arrival_time(1.0, 0.1, 0.0, 4.0, 3.0);
    assert!((t - 10.017).abs() < 0.01, "got {t}");
}

#[test]
fn arrival_time_always_positive() {
    // Sanity check for various scenarios
    assert!(arrival_time(100.0, 5.0, 15.0, 4.0, 3.0) > 0.0);
    assert!(arrival_time(100.0, 15.0, 5.0, 4.0, 3.0) > 0.0);
    assert!(arrival_time(0.001, 1.0, 1.0, 4.0, 3.0) > 0.0);
}

// ---- leave_time ----

#[test]
fn leave_time_constant_speed() {
    // t_arrive=5, lane=20, veh=10, v_arrive=10, v_leave=10 → avg=10
    // result = 5 + 30/10 = 8.0
    let t = leave_time(5.0, 20.0, 10.0, 10.0, 10.0);
    assert!((t - 8.0).abs() < 1e-4, "expected 8.0, got {t}");
}

#[test]
fn leave_time_accelerating() {
    // t=0, lane=20, veh=5, v_arrive=5, v_leave=15 → avg=10
    // result = 0 + 25/10 = 2.5
    let t = leave_time(0.0, 20.0, 5.0, 5.0, 15.0);
    assert!((t - 2.5).abs() < 1e-4, "expected 2.5, got {t}");
}

#[test]
fn leave_time_zero_speeds_no_divide_by_zero() {
    // avg_speed clamped to 0.1 → should not panic, result is finite positive
    let t = leave_time(1.0, 10.0, 5.0, 0.0, 0.0);
    assert!(t.is_finite());
    assert!(t > 1.0);
}

#[test]
fn leave_time_increases_with_lane_length() {
    let t_short = leave_time(0.0, 10.0, 5.0, 10.0, 10.0);
    let t_long = leave_time(0.0, 50.0, 5.0, 10.0, 10.0);
    assert!(t_long > t_short);
}

// ---- v_stop_at ----

#[test]
fn v_stop_at_zero_distance() {
    assert_eq!(v_stop_at(0.0, 3.0), 0.0);
}

#[test]
fn v_stop_at_basic() {
    // sqrt(2 * 3.0 * 50.0) = sqrt(300) ≈ 17.3205
    let v = v_stop_at(50.0, 3.0);
    assert!((v - 300.0f32.sqrt()).abs() < 1e-3, "got {v}");
}

#[test]
fn v_stop_at_scaling_with_deceleration() {
    // Doubling d_max multiplies v by sqrt(2)
    let v1 = v_stop_at(50.0, 3.0);
    let v2 = v_stop_at(50.0, 6.0);
    let ratio = v2 / v1;
    assert!((ratio - 2.0f32.sqrt()).abs() < 1e-3, "ratio {ratio}");
}

#[test]
fn v_stop_at_negative_distance_returns_zero() {
    assert_eq!(v_stop_at(-5.0, 3.0), 0.0);
}

// ---- approach_speed ----

#[test]
fn approach_speed_priority() {
    assert_eq!(approach_speed(&LinkType::Priority, 40.0), 40.0);
}

#[test]
fn approach_speed_yield() {
    let v = approach_speed(&LinkType::Yield, 40.0);
    assert!((v - 28.0).abs() < 1e-4, "expected 28.0, got {v}");
}

#[test]
fn approach_speed_stop() {
    assert_eq!(approach_speed(&LinkType::Stop, 40.0), 0.0);
}

#[test]
fn approach_speed_traffic_light() {
    assert_eq!(approach_speed(&LinkType::TrafficLight, 40.0), 40.0);
}
