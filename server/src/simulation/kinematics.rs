use crate::map::road::LinkType;
use crate::simulation::config::MIN_CREEP_SPEED;

/// Minimum time to drive `dist` starting at `v0` and aiming for `v1` at the end, plus the
/// speed actually reached. Speeds up at `a_max` (then cruises) or cruises then brakes at
/// `d_max`. When `dist` is too short to reach `v1`, the reachable speed is returned.
pub fn arrival(dist: f32, v0: f32, v1: f32, a_max: f32, d_max: f32) -> (f32, f32) {
    debug_assert!(a_max > 0.0 && d_max > 0.0);

    if dist <= 0.0 {
        return (0.0, v0);
    }

    if v1 >= v0 {
        // A vehicle always moves eventually: never plan to cruise slower than a creep.
        let v1 = v1.max(MIN_CREEP_SPEED);
        let t_accel = (v1 - v0) / a_max;
        let d_accel = t_accel * (v0 + v1) / 2.0;

        if dist >= d_accel {
            (t_accel + (dist - d_accel) / v1, v1)
        } else {
            let v_end = (v0 * v0 + 2.0 * a_max * dist).sqrt();
            ((v_end - v0) / a_max, v_end)
        }
    } else {
        let t_decel = (v0 - v1) / d_max;
        let d_decel = t_decel * (v0 + v1) / 2.0;

        if dist >= d_decel {
            (t_decel + (dist - d_decel) / v0, v1)
        } else {
            // Too close to slow down to v1: arrive faster than planned.
            let v_end = (v0 * v0 - 2.0 * d_max * dist).max(0.0).sqrt();
            ((v0 - v_end) / d_max, v_end)
        }
    }
}

pub fn arrival_time(dist: f32, v0: f32, v1: f32, a_max: f32, d_max: f32) -> f32 {
    arrival(dist, v0, v1, a_max, d_max).0
}

/// Time needed to drive `dist` starting at `v0` and accelerating at `a_max` up to `v_max`.
pub fn travel_time(dist: f32, v0: f32, v_max: f32, a_max: f32) -> f32 {
    arrival(dist, v0, v_max.max(v0), a_max, a_max).0
}

/// Time at which a vehicle entering a junction lane at `t_arrive` has fully left it:
/// its front drives the lane plus its own length while accelerating towards `v_leave`.
pub fn leave_time(
    t_arrive: f32,
    lane_len: f32,
    veh_len: f32,
    v_arrive: f32,
    v_leave: f32,
    a_max: f32,
) -> f32 {
    t_arrive + travel_time(lane_len + veh_len, v_arrive, v_leave, a_max)
}

/// Highest speed from which a vehicle can still brake to `v_target` within `dist`.
pub fn v_approach(dist: f32, v_target: f32, d_max: f32) -> f32 {
    debug_assert!(d_max > 0.0);
    (v_target * v_target + 2.0 * d_max * dist.max(0.0)).sqrt()
}

/// Distance needed to brake from `v` to a standstill.
pub fn braking_distance(v: f32, d_max: f32) -> f32 {
    v * v / (2.0 * d_max)
}

pub fn approach_speed(link_type: &LinkType, road_speed_limit: f32) -> f32 {
    match link_type {
        LinkType::Priority => road_speed_limit,
        LinkType::Yield => road_speed_limit * 0.7,
        LinkType::Stop => 0.0,
        LinkType::TrafficLight => road_speed_limit,
    }
}
