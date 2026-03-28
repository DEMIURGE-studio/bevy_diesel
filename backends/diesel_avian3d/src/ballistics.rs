use bevy::prelude::*;

/// Low-angle ballistic launch velocity. Falls back to 45 degrees if out of range.
pub fn calculate_low_angle_velocity_with_speed(
    origin: Vec3,
    target: Vec3,
    speed: f32,
    gravity: f32,
) -> Vec3 {
    let (horizontal_direction, horizontal_distance, height_difference) =
        decompose_trajectory(origin, target);

    let launch_angle =
        solve_launch_angle(horizontal_distance, height_difference, speed, gravity, false);

    compose_velocity(horizontal_direction, launch_angle, speed)
}

/// High-angle ballistic launch velocity. Falls back to 45 degrees if out of range.
pub fn calculate_high_angle_velocity_with_speed(
    origin: Vec3,
    target: Vec3,
    speed: f32,
    gravity: f32,
) -> Vec3 {
    let (horizontal_direction, horizontal_distance, height_difference) =
        decompose_trajectory(origin, target);

    let launch_angle =
        solve_launch_angle(horizontal_distance, height_difference, speed, gravity, true);

    compose_velocity(horizontal_direction, launch_angle, speed)
}

/// Alias for `calculate_low_angle_velocity_with_speed`.
pub fn calculate_velocity_with_speed(
    origin: Vec3,
    target: Vec3,
    speed: f32,
    gravity: f32,
) -> Vec3 {
    calculate_low_angle_velocity_with_speed(origin, target, speed, gravity)
}

/// Clamp target position to `[min, max]` distance from origin.
pub fn distance_lock(origin: Vec3, target: Vec3, min: f32, max: f32) -> Vec3 {
    let distance = origin.distance(target);
    if distance >= min && distance <= max {
        return target;
    }

    let velocity_to_target = target - origin;
    velocity_to_target.clamp_length(min, max) + origin
}

// ---- internal helpers ----

fn decompose_trajectory(origin: Vec3, target: Vec3) -> (Vec2, f32, f32) {
    let planar_origin = origin.xz();
    let planar_target = target.xz();

    let horizontal_distance = planar_origin.distance(planar_target);
    let height_difference = target.y - origin.y;

    let horizontal_direction = if horizontal_distance > 0.001 {
        (planar_target - planar_origin).normalize()
    } else {
        Vec2::new(0.0, 1.0)
    };

    (horizontal_direction, horizontal_distance, height_difference)
}

fn solve_launch_angle(
    horizontal_distance: f32,
    height_difference: f32,
    speed: f32,
    gravity: f32,
    high_angle: bool,
) -> f32 {
    let speed_squared = speed * speed;
    let g_term = gravity * horizontal_distance * horizontal_distance / (2.0 * speed_squared);

    let a = g_term;
    let b = -horizontal_distance;
    let c = g_term + height_difference;

    let discriminant = b * b - 4.0 * a * c;

    if discriminant >= 0.0 && a.abs() > 0.001 {
        let tan_theta_1 = (-b - discriminant.sqrt()) / (2.0 * a);
        let tan_theta_2 = (-b + discriminant.sqrt()) / (2.0 * a);

        let angle_1 = tan_theta_1.atan();
        let angle_2 = tan_theta_2.atan();

        if high_angle {
            angle_1.max(angle_2)
        } else {
            angle_1.min(angle_2)
        }
    } else {
        std::f32::consts::PI / 4.0
    }
}

fn compose_velocity(horizontal_direction: Vec2, launch_angle: f32, speed: f32) -> Vec3 {
    let velocity_horizontal = speed * launch_angle.cos();
    let velocity_vertical = speed * launch_angle.sin();

    Vec3::new(
        horizontal_direction.x * velocity_horizontal,
        velocity_vertical,
        horizontal_direction.y * velocity_horizontal,
    )
}
