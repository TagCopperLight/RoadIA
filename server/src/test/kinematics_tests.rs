use crate::map::road::LinkType;
use crate::simulation::kinematics::{
    approach_speed, arrival, arrival_time, braking_distance, leave_time, v_approach,
};

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

// ---- arrival ----

#[test]
fn arrival_from_standstill_with_zero_target_is_finite() {
    // A vehicle standing at a stop line (target speed 0) must still get a finite
    // arrival time, otherwise its foes never see it coming.
    let (t, v) = arrival(2.0, 0.0, 0.0, 4.0, 3.0);
    assert!(t.is_finite() && t > 0.0, "got {t}");
    assert!(v > 0.0);
}

#[test]
fn arrival_reports_reachable_speed_when_too_short_to_accelerate() {
    // 1 m from standstill at 4 m/s²: v = sqrt(2 * 4 * 1)
    let (_, v) = arrival(1.0, 0.0, 10.0, 4.0, 3.0);
    assert!((v - 8.0f32.sqrt()).abs() < 1e-3, "got {v}");
}

#[test]
fn arrival_reports_speed_when_too_short_to_brake() {
    // v0=10, v1=5, 10 m at 3 m/s²: v = sqrt(100 - 60)
    let (_, v) = arrival(10.0, 10.0, 5.0, 4.0, 3.0);
    assert!((v - 40.0f32.sqrt()).abs() < 1e-3, "got {v}");
}

// ---- leave_time ----

#[test]
fn leave_time_constant_speed() {
    // lane=20, veh=10, cruising at 10 m/s → 3 s after arrival
    let t = leave_time(5.0, 20.0, 10.0, 10.0, 10.0, 4.0);
    assert!((t - 8.0).abs() < 1e-4, "expected 8.0, got {t}");
}

#[test]
fn leave_time_accelerating() {
    // From 0 m/s at 4 m/s², 8 m are covered in 2 s (top speed 10 not reached).
    let t = leave_time(0.0, 3.0, 5.0, 0.0, 10.0, 4.0);
    assert!((t - 2.0).abs() < 1e-4, "expected 2.0, got {t}");
}

#[test]
fn leave_time_from_standstill_is_finite() {
    let t = leave_time(1.0, 10.0, 5.0, 0.0, 0.0, 4.0);
    assert!(t.is_finite());
    assert!(t > 1.0);
}

#[test]
fn leave_time_increases_with_lane_length() {
    let t_short = leave_time(0.0, 10.0, 5.0, 10.0, 10.0, 4.0);
    let t_long = leave_time(0.0, 50.0, 5.0, 10.0, 10.0, 4.0);
    assert!(t_long > t_short);
}

// ---- v_approach / braking_distance ----

#[test]
fn v_approach_to_a_stop() {
    // sqrt(2 * 3.0 * 50.0) = sqrt(300)
    let v = v_approach(50.0, 0.0, 3.0);
    assert!((v - 300.0f32.sqrt()).abs() < 1e-3, "got {v}");
}

#[test]
fn v_approach_at_the_line_is_the_target_speed() {
    assert_eq!(v_approach(0.0, 7.0, 3.0), 7.0);
    assert_eq!(v_approach(-5.0, 7.0, 3.0), 7.0);
}

#[test]
fn braking_distance_matches_v_approach() {
    let d = braking_distance(12.0, 3.0);
    assert!((v_approach(d, 0.0, 3.0) - 12.0).abs() < 1e-3);
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
