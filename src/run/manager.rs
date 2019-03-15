// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

use run::compile::ExpressionContext;

/// The implementation of a bullet.
///
/// This trait is driven by the `Runner` structure to perform the actions indicated by the
/// BulletML script.
pub trait BulletManager: ExpressionContext {
    /// Create a new, simple, bullet.
    fn new_simple(&mut self, direction: f32, speed: f32);
    /// Create a new bullet.
    fn new(&mut self, direction: f32, speed: f32);
    /// The turn of the simulation.
    fn turn(&self) -> u32;

    /// The current direction of the bullet.
    fn direction(&self) -> f32;
    /// The direction the bullet should aim for.
    fn aim_direction(&self) -> f32;
    /// The current speed of the bullet.
    fn speed(&self) -> f32;
    /// The current `x`-axis speed of the bullet.
    fn speed_x(&self) -> f32;
    /// The current `y`-axis speed of the bullet.
    fn speed_y(&self) -> f32;
    /// The default speed of the bullet.
    fn default_speed(&self) -> f32;

    /// Destroy the bullet.
    fn vanish(&mut self);
    /// Change the direction of the bullet.
    fn change_direction(&mut self, degrees: f32);
    /// Change the speed of the bullet.
    fn change_speed(&mut self, speed: f32);
    /// Accelerate the bullet along the `x` axis.
    fn accel_x(&mut self, amount: f32);
    /// Accelerate the bullet along the `y` axis.
    fn accel_y(&mut self, amount: f32);
}
