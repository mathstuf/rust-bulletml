// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying file LICENSE for details.

//! Data entities
//!
//! These are the data structures used to represent a BulletML file.

use std::collections::hash_map::HashMap;
use std::ops::{Add, Mul};
use std::rc::Rc;

use error::*;

mod expression;
pub use self::expression::{Expression, ExpressionContext, Value};

/// Cause acceleration of a bullet for a given about of time.
pub struct Accel {
    /// The amount to accelerate along the horizontal axis.
    pub horizontal: Option<Horizontal>,
    /// The amount to accelerate along the vertical axis.
    pub vertical: Option<Vertical>,
    /// The number of frames to accelerate.
    pub duration: Term,
}

/// Entities which may appear within an action.
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
pub struct Action {
    /// The name of the action.
    pub label: String,
    /// The steps which make up the action.
    pub steps: Vec<Step>,
}

/// A bullet.
pub struct Bullet {
    /// The label for the bullet.
    pub label: String,
    /// The direction to fire the bullet.
    pub direction: Option<Direction>,
    /// The initial speed of the bullet.
    pub speed: Option<Speed>,
    /// The set of actions to perform on the bullet.
    pub actions: Vec<EntityRef<Action>>,
}

/// The orientation of the game.
pub enum Orientation {
    /// For games with a toroidal topology.
    None,
    /// For games with a vertical orientation.
    Vertical,
    /// For games with a horizontal orientation.
    Horizontal,
}

impl Default for Orientation {
    fn default() -> Self {
        Orientation::None
    }
}

/// Elements allowed at the top-level of the structure.
pub enum Element {
    /// A bullet entity.
    Bullet(Rc<Bullet>),
    /// An action entity.
    Action(Rc<Action>),
    /// A fire entity.
    Fire(Rc<Fire>),
}

/// The top-level BulletML entity.
pub struct BulletML {
    /// The orientation of the game.
    pub orientation: Orientation,
    /// The elements which make up the entity.
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Ways a value may change.
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
    pub fn modify<T>(&self, value: T, current: T, duration: T) -> T
        where T: Add<Output = T>,
              T: Mul<Output = T>,
    {
        match *self {
            Change::Absolute => value,
            Change::Relative => value + current,
            Change::Sequence => value * duration + current,
        }
    }
}

/// A change in direction.
pub struct ChangeDirection {
    /// The direction to change.
    pub direction: Direction,
    /// How much to change the direction by.
    pub value: Term,
}

/// A change in speed.
pub struct ChangeSpeed {
    /// The speed to change.
    pub speed: Speed,
    /// How much to change the speed by.
    pub value: Term,
}

/// How to interpret a direction.
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
pub struct Direction {
    /// What kind of direction is given.
    pub kind: DirectionKind,
    /// The angle against the given direction.
    pub degrees: Expression,
}

/// A reference to a given entity.
pub enum EntityRef<T> {
    /// A named entity.
    Ref(String),
    /// An actual entity.
    Real(Rc<T>),
}

impl<T> EntityRef<T> {
    /// Get a reference to the entity.
    pub fn entity<'a>(&'a self, lookup: &'a HashMap<String, Rc<T>>) -> Result<&'a T> {
        match *self {
            EntityRef::Ref(ref label) => {
                lookup.get(label)
                    .map(AsRef::as_ref)
                    .ok_or_else(|| ErrorKind::NoSuchEntity(label.to_string()).into())
            },
            EntityRef::Real(ref rc) => Ok(&rc),
        }
    }
}

/// Create a new bullet.
pub struct Fire {
    /// The name of the fire action.
    pub label: String,
    /// The direction to fire in.
    pub direction: Option<Direction>,
    /// The initial speed of the bullet.
    pub speed: Option<Speed>,
    /// The bullet to fire.
    pub bullet: EntityRef<Bullet>,
}

/// Horizontal change description.
pub struct Horizontal {
    /// How to change horizontally.
    pub kind: Change,
    /// How much to change by.
    pub change: Expression,
}

/// Repetition action.
pub struct Repeat {
    /// How many times to repeat the actions.
    pub times: Times,
    /// The actions to repeat.
    pub actions: Vec<EntityRef<Action>>,
}

/// A change in speed.
pub struct Speed {
    /// How to change the speed.
    pub kind: Change,
    /// How much to change the speed by.
    pub change: Expression,
}

/// An expression to compute a value for an action.
pub struct Term {
    /// The value of the term.
    pub value: Expression,
}

/// A count of how many times to repeat an action.
pub struct Times {
    /// How many times to repeat an action.
    pub value: Expression,
}

/// Cause the bullet to vanish.
pub struct Vanish;

/// Vertical change description.
pub struct Vertical {
    /// How to change vertically.
    pub kind: Change,
    /// How much to change by.
    pub change: Expression,
}

/// Pause execution for a given number of frames.
pub struct Wait {
    /// The number of frames to wait for.
    pub frames: Expression,
}
