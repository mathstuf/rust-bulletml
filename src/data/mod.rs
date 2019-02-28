// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

//! Data entities
//!
//! These are the data structures used to represent a BulletML file.

use std::ops::{Add, Mul};
use std::rc::Rc;

use crates::failure::Fallible;

/// An error related to entity searches.
#[derive(Debug, Fail)]
pub enum EntityError {
    /// An entity with the given name could not be found.
    #[fail(display = "could not find entity `{}`", _0)]
    CannotFind(String),
}

mod expression;
pub use self::expression::{Expression, ExpressionContext, Value};

/// Cause acceleration of a bullet for a given about of time.
#[derive(Debug, Clone)]
pub struct Accel {
    /// The amount to accelerate along the horizontal axis.
    pub horizontal: Option<Horizontal>,
    /// The amount to accelerate along the vertical axis.
    pub vertical: Option<Vertical>,
    /// The number of frames to accelerate.
    pub duration: Term,
}

/// Entities which may appear within an action.
#[derive(Debug)]
pub enum Step {
    /// Cause a set of actions to be repeated a number of times.
    Repeat(Repeat),
    /// Cause a set bullets to be fired.
    Fire(EntityRef<Fire>),
    /// A change of speed.
    ChangeSpeed(ChangeSpeed),
    /// A change of direction.
    ChangeDirection(ChangeDirection),
    /// An acceleration.
    Accel(Accel),
    /// Pause for a number of frames.
    Wait(Wait),
    /// Destroy the bullet.
    Vanish(Vanish),
    /// Chain into another action.
    Action(EntityRef<Action>),
}

/// An action that may be performed for a bullet.
#[derive(Debug)]
pub struct Action {
    /// The name of the action.
    pub label: Option<String>,
    /// The steps which make up the action.
    pub steps: Vec<Step>,
}

/// A bullet.
#[derive(Debug)]
pub struct Bullet {
    /// The label for the bullet.
    pub label: Option<String>,
    /// The direction to fire the bullet.
    pub direction: Option<Direction>,
    /// The initial speed of the bullet.
    pub speed: Option<Speed>,
    /// The set of actions to perform on the bullet.
    pub actions: Vec<EntityRef<Action>>,
}

/// The orientation of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// For games with a toroidal topology.
    None,
    /// For games with a vertical orientation.
    Vertical,
    /// For games with a horizontal orientation.
    Horizontal,
}

impl Orientation {
    /// The "up" direction for the given orientation.
    pub fn up(self, dir: f32) -> f32 {
        if let Orientation::Horizontal = self {
            dir - 90.
        } else {
            dir
        }
    }
}

impl Default for Orientation {
    fn default() -> Self {
        Orientation::None
    }
}

/// Elements allowed at the top-level of the structure.
#[derive(Debug, Clone)]
pub enum Element {
    /// A bullet entity.
    Bullet(Rc<Bullet>),
    /// An action entity.
    Action(Rc<Action>),
    /// A fire entity.
    Fire(Rc<Fire>),
}

/// The top-level BulletML entity.
#[derive(Debug, Clone)]
pub struct BulletML {
    /// The orientation of the game.
    pub orientation: Orientation,
    /// The elements which make up the entity.
    pub elements: Vec<Element>,
}

/// Ways a value may change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Set the value.
    Absolute,
    /// Set the value relative to the current value.
    Relative,
    /// Set the value relative to the current value over time.
    Sequence,
}

impl Default for Change {
    fn default() -> Self {
        Change::Absolute
    }
}

impl Change {
    /// Change a value.
    pub fn modify<T>(self, value: T, current: T, duration: T) -> T
    where
        T: Add<Output = T>,
        T: Mul<Output = T>,
    {
        match self {
            Change::Absolute => value,
            Change::Relative => value + current,
            Change::Sequence => value * duration + current,
        }
    }
}

/// A change in direction.
#[derive(Debug, Clone)]
pub struct ChangeDirection {
    /// The direction to change.
    pub direction: Direction,
    /// How much to change the direction by.
    pub value: Term,
}

/// A change in speed.
#[derive(Debug, Clone)]
pub struct ChangeSpeed {
    /// The speed to change.
    pub speed: Speed,
    /// How much to change the speed by.
    pub value: Term,
}

/// How to interpret a direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectionKind {
    /// Aim towards the player.
    Aim,
    /// Go to an absolute location.
    Absolute,
    /// Go relative to the current heading.
    Relative,
    /// Increment by a given amount each frame.
    Sequence,
}

impl Default for DirectionKind {
    fn default() -> Self {
        DirectionKind::Aim
    }
}

/// The direction of a bullet.
#[derive(Debug, Clone)]
pub struct Direction {
    /// What kind of direction is given.
    pub kind: DirectionKind,
    /// The angle against the given direction.
    pub degrees: Expression,
}

/// A reference to a given entity.
#[derive(Debug, Clone)]
pub enum EntityRef<T> {
    /// A named entity.
    Ref(String),
    /// An actual entity.
    Real(Rc<T>),
}

/// A trait to look up entities.
pub trait EntityLookup<T> {
    /// Find an entity by name.
    fn find(&self, name: &str) -> Option<Rc<T>>;
}

impl<T> EntityRef<T> {
    /// Get a reference to the entity.
    pub fn entity(&self, lookup: &EntityLookup<T>) -> Result<Rc<T>, EntityError> {
        match *self {
            EntityRef::Ref(ref label) => {
                lookup
                    .find(&label)
                    .ok_or_else(|| EntityError::CannotFind(label.clone()))
            },
            EntityRef::Real(ref rc) => Ok(rc.clone()),
        }
    }
}

/// Create a new bullet.
#[derive(Debug)]
pub struct Fire {
    /// The name of the fire action.
    pub label: Option<String>,
    /// The direction to fire in.
    pub direction: Option<Direction>,
    /// The initial speed of the bullet.
    pub speed: Option<Speed>,
    /// The bullet to fire.
    pub bullet: EntityRef<Bullet>,
}

/// Horizontal change description.
#[derive(Debug, Clone)]
pub struct Horizontal {
    /// How to change horizontally.
    pub kind: Change,
    /// How much to change by.
    pub change: Expression,
}

/// Repetition action.
#[derive(Debug)]
pub struct Repeat {
    /// How many times to repeat the actions.
    pub times: Times,
    /// The actions to repeat.
    pub actions: Vec<EntityRef<Action>>,
}

/// A change in speed.
#[derive(Debug, Clone)]
pub struct Speed {
    /// How to change the speed.
    pub kind: Change,
    /// How much to change the speed by.
    pub change: Expression,
}

/// An expression to compute a value for an action.
#[derive(Debug, Clone)]
pub struct Term {
    /// The value of the term.
    pub value: Expression,
}

impl Term {
    /// Evaluate the term in the given context.
    pub fn eval(&self, ctx: &ExpressionContext) -> Fallible<Value> {
        self.value.eval(ctx)
    }
}

/// A count of how many times to repeat an action.
#[derive(Debug, Clone)]
pub struct Times {
    /// How many times to repeat an action.
    pub value: Expression,
}

/// Cause the bullet to vanish.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vanish;

/// Vertical change description.
#[derive(Debug, Clone)]
pub struct Vertical {
    /// How to change vertically.
    pub kind: Change,
    /// How much to change by.
    pub change: Expression,
}

/// Pause execution for a given number of frames.
#[derive(Debug, Clone)]
pub struct Wait {
    /// The number of frames to wait for.
    pub frames: Expression,
}
