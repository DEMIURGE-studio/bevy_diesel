use bevy::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::*; // TODO: Update specific imports for bevy_rand 0.11
use rand::RngCore;

// Type Aliases
pub type Rng = Entropy<WyRand>;
pub type GlobalRng<'a> = GlobalEntropy<'a, WyRand>;

// Trait with Default Implementations
pub(crate) trait RngExt: RngCore {
    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    fn f32_in_range(&mut self, range: std::ops::Range<f32>) -> f32 {
        let range_size = range.end - range.start;
        range.start + self.next_f32() * range_size
    }

    fn f32_around_zero(&mut self, distance: f32) -> f32 {
        self.f32_in_range(-distance..distance)
    }

    fn next_f64(&mut self) -> f64 {
        self.next_u64() as f64 / u64::MAX as f64
    }

    fn f64_in_range(&mut self, range: std::ops::Range<f64>) -> f64 {
        let range_size = range.end - range.start;
        range.start + self.next_f64() * range_size
    }

    fn next_i32(&mut self) -> i32 {
        (self.next_u32() as i32).abs()
    }

    fn i32_in_range(&mut self, range: std::ops::Range<i32>) -> i32 {
        let range_size = range.end - range.start;
        range.start + (self.next_u32() % range_size as u32) as i32
    }

    fn i32_in_range_inclusive(&mut self, range: std::ops::RangeInclusive<i32>) -> i32 {
        let range_size = range.end() - range.start();
        if range_size == 0 {
            return *range.start(); // If start == end, just return that value
        }
        range.start() + (self.next_u32() % (range_size + 1) as u32) as i32
    }

    fn u32_in_range(&mut self, range: std::ops::RangeInclusive<u32>) -> u32 {
        let range_size = range.end() - range.start();
        if range_size == 0 {
            return *range.start(); // If start == end, just return that value
        }
        range.start() + (self.next_u32() % (range_size + 1))
    }

    fn usize_in_range(&mut self, range: std::ops::RangeInclusive<usize>) -> usize {
        let range_size = range.end() - range.start();
        if range_size == 0 {
            return *range.start(); // If start == end, just return that value
        }
        range.start() + (self.next_u32() % (range_size + 1) as u32) as usize
    }

    fn f32_in_range_inclusive(&mut self, range: std::ops::RangeInclusive<f32>) -> f32 {
        let range_size = range.end() - range.start();
        range.start() + self.next_f32() * range_size
    }

    fn random_vec2(&mut self, range: std::ops::Range<f32>) -> Vec2 {
        Vec2::new(
            self.f32_in_range(range.clone()),
            self.f32_in_range(range),
        )
    }

    fn random_vec2_around_zero(&mut self, distance: f32) -> Vec2 {
        Vec2::new(
            self.f32_around_zero(distance),
            self.f32_around_zero(distance),
        )
    }

    fn random_normal_vec2(&mut self) -> Vec2 {
        let angle = self.next_f32() * std::f32::consts::TAU;
        Vec2::new(angle.cos(), angle.sin())
    }

    fn random_vec3(&mut self, range: std::ops::Range<f32>) -> Vec3 {
        Vec3::new(
            self.f32_in_range(range.clone()),
            self.f32_in_range(range.clone()),
            self.f32_in_range(range),
        )
    }

    fn random_vec3_around_zero(&mut self, distance: f32) -> Vec3 {
        Vec3::new(
            self.f32_around_zero(distance),
            self.f32_around_zero(distance),
            self.f32_around_zero(distance),
        )
    }

    fn random_normal_vec3(&mut self) -> Vec3 {
        let z = self.next_f32() * 2.0 - 1.0;
        let theta = self.next_f32() * std::f32::consts::TAU;
        let r = (1.0 - z * z).sqrt();

        Vec3::new(r * theta.cos(), r * theta.sin(), z)
    }
}

impl<T: RngCore + ?Sized> RngExt for T {}