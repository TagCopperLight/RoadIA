use crate::map::road::LinkType;

pub fn arrival_time(dist: f32, v0: f32, v1: f32, a_max: f32, d_max: f32) -> f32 {
    debug_assert!(dist >= 0.0);
    debug_assert!(a_max > 0.0 && d_max > 0.0);

    if dist <= 0.0 {
        return 0.0;
    }

    if v1 >= v0 {
        let a = a_max;
        let t_accel = (v1 - v0) / a;
        let d_accel = t_accel * (v0 + v1) / 2.0;

        if dist >= d_accel {
            let cruise_speed = v0.max(v1);
            t_accel + (dist - d_accel) / cruise_speed
        } else {
            (-v0 + (v0 * v0 + 2.0 * a * dist).sqrt()) / a
        }
    } else {
        let a = d_max;
        let t_decel = (v0 - v1) / a;
        let d_decel = t_decel * (v0 + v1) / 2.0;

        if dist >= d_decel {
            let cruise_speed = v1.max(v0);
            t_decel + (dist - d_decel) / cruise_speed
        } else {
            let discriminant = v0 * v0 - 2.0 * a * dist;
            if discriminant < 0.0 {
                v0 / a
            } else {
                (v0 - discriminant.sqrt()) / a
            }
        }
    }
}

pub fn leave_time(
    t_arrive: f32,
    lane_len: f32,
    veh_len: f32,
    v_arrive: f32,
    v_leave: f32,
) -> f32 {
    let avg_speed = ((v_arrive + v_leave) / 2.0).max(0.1);
    t_arrive + (lane_len + veh_len) / avg_speed
}

pub fn v_stop_at(dist: f32, d_max: f32) -> f32 {
    debug_assert!(d_max > 0.0);
    if dist <= 0.0 {
        return 0.0;
    }
    (2.0 * d_max * dist).sqrt()
}

pub fn approach_speed(link_type: &LinkType, road_speed_limit: f32) -> f32 {
    match link_type {
        LinkType::Priority => road_speed_limit,
        LinkType::Yield => road_speed_limit * 0.7,
        LinkType::Stop => 0.0,
        LinkType::TrafficLight => road_speed_limit,
    }
}