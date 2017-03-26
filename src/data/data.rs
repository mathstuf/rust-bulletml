// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying file LICENSE for details.

extern crate serde;
use self::serde::de::{self, Deserialize, Deserializer};

use std::collections::hash_map::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Add, Mul};
use std::rc::Rc;

use error::*;

pub use data::expression::Expression;

#[derive(Debug, Clone, Deserialize)]
/// Cause acceleration of a bullet for a given about of time.
pub struct Accel {
    /// The amount to accelerate along the horizontal axis.
    pub horizontal: Option<Horizontal>,
    /// The amount to accelerate along the vertical axis.
    pub vertical: Option<Vertical>,
    /// The number of frames to accelerate.
    pub duration: Term,
}

#[derive(Debug, Clone, Deserialize)]
/// Entities which may appear within an action.
pub enum Step {
    #[serde(rename="repeat")]
    /// Cause a set of actions to be repeated a number of times.
    Repeat(Repeat),
    #[serde(rename="fire")]
    /// Cause a set bullets to be fired.
    Fire(EntityRef<Fire>),
    #[serde(rename="fireRef")]
    /// Cause a set bullets to be fired.
    FireRef(EntityRef<Fire>),
    #[serde(rename="changeSpeed")]
    /// A change of speed.
    ChangeSpeed(ChangeSpeed),
    #[serde(rename="changeDirection")]
    /// A change of direction.
    ChangeDirection(ChangeDirection),
    #[serde(rename="accel")]
    /// An acceleration.
    Accel(Accel),
    #[serde(rename="wait")]
    /// Pause for a number of frames.
    Wait(Wait),
    #[serde(rename="vanish")]
    /// Destroy the bullet.
    Vanish(Vanish),
    #[serde(rename="action")]
    /// Chain into another action.
    Action(EntityRef<Action>),
    #[serde(rename="actionRef")]
    /// Chain into another action.
    ActionRef(EntityRef<Action>),
}

#[derive(Debug, Clone, Deserialize)]
/// An action that may be performed for a bullet.
pub struct Action {
    /// The name of the action.
    pub label: String,
    #[serde(rename="$value")]
    /// The steps which make up the action.
    pub steps: Vec<Step>,
}

const ACTION_REAL_TAG_NAME: &'static str = "action";
const ACTION_REF_TAG_NAME: &'static str = "actionRef";
const ACTION_TAG_NAMES: &'static [&'static str] = &[ACTION_REAL_TAG_NAME, ACTION_REF_TAG_NAME];

impl DeserializeRef for Action {
    fn name() -> &'static str {
        "Action"
    }

    fn names() -> &'static [&'static str] {
        ACTION_TAG_NAMES
    }

    fn real_name() -> &'static str {
        ACTION_REAL_TAG_NAME
    }

    fn ref_name() -> &'static str {
        ACTION_REF_TAG_NAME
    }
}

#[derive(Debug, Clone, Deserialize)]
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

const BULLET_REAL_TAG_NAME: &'static str = "bullet";
const BULLET_REF_TAG_NAME: &'static str = "bulletRef";
const BULLET_TAG_NAMES: &'static [&'static str] = &[BULLET_REAL_TAG_NAME, BULLET_REF_TAG_NAME];

impl DeserializeRef for Bullet {
    fn name() -> &'static str {
        "Bullet"
    }

    fn names() -> &'static [&'static str] {
        BULLET_TAG_NAMES
    }

    fn real_name() -> &'static str {
        BULLET_REAL_TAG_NAME
    }

    fn ref_name() -> &'static str {
        BULLET_REF_TAG_NAME
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
/// The orientation of the game.
pub enum Orientation {
    #[serde(rename="none")]
    /// For games with a toroidal topology.
    None,
    #[serde(rename="vertical")]
    /// For games with a vertical orientation.
    Vertical,
    #[serde(rename="horizontal")]
    /// For games with a horizontal orientation.
    Horizontal,
}

impl Default for Orientation {
    fn default() -> Self {
        Orientation::None
    }
}

#[derive(Debug, Clone, Deserialize)]
/// Elements allowed at the top-level of the structure.
pub enum Element {
    #[serde(rename="bullet")]
    /// A bullet entity.
    Bullet(Rc<Bullet>),
    #[serde(rename="action")]
    /// An action entity.
    Action(Rc<Action>),
    #[serde(rename="fire")]
    /// A fire entity.
    Fire(Rc<Fire>),
}

#[derive(Debug, Clone, Deserialize)]
/// The top-level BulletML entity.
pub struct BulletML {
    #[serde(rename="type")]
    /// The orientation of the game.
    pub orientation: Orientation,
    #[serde(rename="$value")]
    /// The elements which make up the entity.
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
/// A change in direction.
pub struct ChangeDirection {
    /// The direction to change.
    pub direction: Direction,
    /// How much to change the direction by.
    pub value: Term,
}

#[derive(Debug, Clone, Deserialize)]
/// A change in speed.
pub struct ChangeSpeed {
    /// The speed to change.
    pub speed: Speed,
    /// How much to change the speed by.
    pub value: Term,
}

#[derive(Debug, Clone, Copy, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
/// The direction of a bullet.
pub struct Direction {
    #[serde(default, rename="type")]
    /// What kind of direction is given.
    pub kind: DirectionKind,
    #[serde(rename="$value")]
    /// The angle against the given direction.
    pub degrees: Expression,
}

#[derive(Debug, Clone)]
/// A reference to a given entity.
pub enum EntityRef<T> {
    /// An actual entity.
    Real(Rc<T>),
    /// A named entity.
    Ref(String),
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

/// A trait to help deserialize reference elements.
pub trait DeserializeRef: Deserialize {
    /// The name of the element.
    fn name() -> &'static str;
    /// The name of the element.
    fn names() -> &'static [&'static str];
    /// The name of the element used for fully-specified elements.
    fn real_name() -> &'static str;
    /// The name of the element used for referential elements.
    fn ref_name() -> &'static str;
}

struct EntityRefVisitor<T> {
    _data: PhantomData<T>,
}

impl<T> EntityRefVisitor<T> {
    fn new() -> Self {
        EntityRefVisitor {
            _data: PhantomData,
        }
    }
}

impl<T> de::Visitor for EntityRefVisitor<T>
    where T: DeserializeRef,
{
    type Value = EntityRef<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "either a {} or {} value", T::real_name(), T::ref_name())
    }

    fn visit_enum<V>(self, visitor: V) -> ::std::result::Result<Self::Value, V::Error>
        where V: de::EnumVisitor,
    {
        use self::serde::de::{Error, VariantVisitor};
        let (value, visitor): (String, _) = visitor.visit_variant()?;
        if value == T::real_name() {
            visitor.visit_newtype::<T>()
                .map(Rc::new)
                .map(EntityRef::Real)
        } else if value == T::ref_name() {
            visitor.visit_newtype::<String>()
                .map(EntityRef::Ref)
        } else {
            Err(Error::invalid_type(de::Unexpected::Str(&value), &"entity ref variant"))
        }
    }
}

impl<T> Deserialize for EntityRef<T>
    where T: DeserializeRef,
{
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where D: Deserializer,
    {
        deserializer.deserialize_enum(T::name(),
                                      T::names(),
                                      EntityRefVisitor::new())
    }
}

#[derive(Debug, Clone, Deserialize)]
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

const FIRE_REAL_TAG_NAME: &'static str = "fire";
const FIRE_REF_TAG_NAME: &'static str = "fireRef";
const FIRE_TAG_NAMES: &'static [&'static str] = &[FIRE_REAL_TAG_NAME, FIRE_REF_TAG_NAME];

impl DeserializeRef for Fire {
    fn name() -> &'static str {
        "Fire"
    }

    fn names() -> &'static [&'static str] {
        FIRE_TAG_NAMES
    }

    fn real_name() -> &'static str {
        FIRE_REAL_TAG_NAME
    }

    fn ref_name() -> &'static str {
        FIRE_REF_TAG_NAME
    }
}

#[derive(Debug, Clone, Deserialize)]
/// Horizontal change description.
pub struct Horizontal {
    /// How to change horizontally.
    pub kind: Change,
    /// How much to change by.
    pub change: Expression,
}

#[derive(Debug, Clone, Deserialize)]
/// Repetition action.
pub struct Repeat {
    /// How many times to repeat the actions.
    pub times: Times,
    #[serde(rename="$value")]
    /// The actions to repeat.
    pub actions: Vec<EntityRef<Action>>,
}

#[derive(Debug, Clone, Deserialize)]
/// A change in speed.
pub struct Speed {
    /// How to change the speed.
    pub kind: Change,
    #[serde(rename="$value")]
    /// How much to change the speed by.
    pub change: Expression,
}

#[derive(Debug, Clone, Deserialize)]
/// An expression to compute a value for an action.
pub struct Term {
    /// The value of the term.
    pub value: Expression,
}

#[derive(Debug, Clone, Deserialize)]
/// A count of how many times to repeat an action.
pub struct Times {
    #[serde(rename="$value")]
    /// How many times to repeat an action.
    pub value: Expression,
}

#[derive(Debug, Clone, Copy, Deserialize)]
/// Cause the bullet to vanish.
pub struct Vanish;

#[derive(Debug, Clone, Deserialize)]
/// Vertical change description.
pub struct Vertical {
    /// How to change vertically.
    pub kind: Change,
    /// How much to change by.
    pub change: Expression,
}

#[derive(Debug, Clone, Deserialize)]
/// Pause execution for a given number of frames.
pub struct Wait {
    #[serde(rename="$value")]
    /// The number of frames to wait for.
    pub frames: Expression,
}
