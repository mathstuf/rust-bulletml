// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Add, Mul};
use std::rc::Rc;

use serde::de::{Deserializer, EnumAccess, Error, MapAccess, VariantAccess, Visitor};
use serde::Deserialize;
use serde_with::enum_map::EnumMap;
use serde_with::serde_as;
use thiserror::Error;

use crate::data::expression::{Expression, ExpressionContext, ExpressionError, Value};

/// An error related to entity searches.
#[derive(Debug, Error)]
pub enum EntityError {
    /// An entity with the given name could not be found.
    #[error("could not find entity `{}`", label)]
    CannotFind {
        /// The label for the requested entity.
        label: String,
    },
}

impl EntityError {
    fn cannot_find(label: String) -> Self {
        Self::CannotFind {
            label,
        }
    }
}

/// Cause acceleration of a bullet for a given about of time.
#[derive(Debug, Clone, Deserialize)]
pub struct Accel {
    /// The amount to accelerate along the horizontal axis.
    pub horizontal: Option<Horizontal>,
    /// The amount to accelerate along the vertical axis.
    pub vertical: Option<Vertical>,
    /// The number of frames to accelerate.
    #[serde(rename = "term")]
    pub duration: Term,
}

/// Entities which may appear within an action.
#[derive(Debug, Clone)]
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

struct StepVisitor;

impl StepVisitor {
    const FIELDS: &'static [&'static str] = &[
        "repeat",
        "fire",
        "fireRef",
        "changeSpeed",
        "changeDirection",
        "accel",
        "wait",
        "vanish",
        "action",
        "actionRef",
    ];
}

impl<'de> Visitor<'de> for StepVisitor {
    type Value = Step;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "one of `{}`", Self::FIELDS.join("`, `"))
    }

    fn visit_enum<E>(self, access: E) -> Result<Self::Value, E::Error>
    where
        E: EnumAccess<'de>,
    {
        let (name, v): (Cow<str>, _) = access.variant()?;
        match name.as_ref() {
            "repeat" => Ok(Step::Repeat(v.newtype_variant()?)),
            "fire" => {
                let fire = v.newtype_variant()?;
                Ok(Step::Fire(EntityRef::Real(Rc::new(fire))))
            },
            "fireRef" => {
                let iref = v.newtype_variant::<Reference>()?;
                Ok(Step::Fire(EntityRef::Ref(iref)))
            },
            "changeSpeed" => Ok(Step::ChangeSpeed(v.newtype_variant()?)),
            "changeDirection" => Ok(Step::ChangeDirection(v.newtype_variant()?)),
            "accel" => Ok(Step::Accel(v.newtype_variant()?)),
            "wait" => Ok(Step::Wait(v.newtype_variant()?)),
            "vanish" => Ok(Step::Vanish(v.newtype_variant()?)),
            "action" => {
                let action = v.newtype_variant()?;
                Ok(Step::Action(EntityRef::Real(Rc::new(action))))
            },
            "actionRef" => {
                let iref = v.newtype_variant::<Reference>()?;
                Ok(Step::Action(EntityRef::Ref(iref)))
            },
            name => Err(E::Error::unknown_variant(name, Self::FIELDS)),
        }
    }
}

impl<'de> Deserialize<'de> for Step {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_enum("Step", StepVisitor::FIELDS, StepVisitor)
    }
}

/// An action that may be performed for a bullet.
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct Action {
    /// The name of the action.
    pub label: Option<String>,
    /// The steps which make up the action.
    #[serde(flatten)]
    #[serde_as(as = "EnumMap")]
    pub steps: Vec<Step>,
}

/// A bullet.
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct Bullet {
    /// The label for the bullet.
    pub label: Option<String>,
    /// The direction to fire the bullet.
    pub direction: Option<Direction>,
    /// The initial speed of the bullet.
    pub speed: Option<Speed>,
    /// The set of actions to perform on the bullet.
    #[serde(default)]
    #[serde(flatten)]
    #[serde_as(as = "EnumMap")]
    pub actions: Vec<EntityRef<Action>>,
}

/// The orientation of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Orientation {
    /// For games with a toroidal topology.
    #[serde(rename = "none")]
    None,
    /// For games with a vertical orientation.
    #[serde(rename = "vertical")]
    Vertical,
    /// For games with a horizontal orientation.
    #[serde(rename = "horizontal")]
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
#[derive(Debug, Clone, Deserialize)]
pub enum Element {
    /// A bullet entity.
    #[serde(rename = "bullet")]
    Bullet(Rc<Bullet>),
    /// An action entity.
    #[serde(rename = "action")]
    Action(Rc<Action>),
    /// A fire entity.
    #[serde(rename = "fire")]
    Fire(Rc<Fire>),
}

/// The top-level BulletML entity.
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct BulletML {
    /// The orientation of the game.
    #[serde(default)]
    #[serde(rename = "type")]
    pub orientation: Orientation,
    /// The elements which make up the entity.
    #[serde(flatten)]
    #[serde_as(as = "EnumMap")]
    pub elements: Vec<Element>,
}

/// Ways a value may change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Change {
    /// Set the value.
    #[serde(rename = "absolute")]
    Absolute,
    /// Set the value relative to the current value.
    #[serde(rename = "relative")]
    Relative,
    /// Set the value relative to the current value over time.
    #[serde(rename = "sequence")]
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
#[derive(Debug, Clone, Deserialize)]
pub struct ChangeDirection {
    /// The direction to change.
    pub direction: Direction,
    /// How much to change the direction by.
    #[serde(rename = "term")]
    pub value: Term,
}

/// A change in speed.
#[derive(Debug, Clone, Deserialize)]
pub struct ChangeSpeed {
    /// The speed to change.
    pub speed: Speed,
    /// How much to change the speed by.
    #[serde(rename = "term")]
    pub value: Term,
}

/// How to interpret a direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum DirectionKind {
    /// Aim towards the player.
    #[serde(rename = "aim")]
    Aim,
    /// Go to an absolute location.
    #[serde(rename = "absolute")]
    Absolute,
    /// Go relative to the current heading.
    #[serde(rename = "relative")]
    Relative,
    /// Increment by a given amount each frame.
    #[serde(rename = "sequence")]
    Sequence,
}

impl Default for DirectionKind {
    fn default() -> Self {
        DirectionKind::Aim
    }
}

/// The direction of a bullet.
#[derive(Debug, Clone, Deserialize)]
pub struct Direction {
    /// What kind of direction is given.
    #[serde(default, rename = "type")]
    pub kind: DirectionKind,
    /// The angle against the given direction.
    #[serde(rename = "$value")]
    pub degrees: Expression,
}

/// A parameter to an entity reference.
#[derive(Debug, Clone, Deserialize)]
pub struct Param {
    /// The expression of the parameter.
    #[serde(rename = "$value")]
    value: Expression,
}

/// A reference to another entity.
#[derive(Debug, Clone)]
pub struct Reference {
    /// The name of the referred-to entity.
    label: String,
    /// Parameters to forward to the entity.
    params: Vec<Param>,
}

struct ReferenceVisitor;

impl ReferenceVisitor {
    const FIELDS: &'static [&'static str] = &["label", "param"];
}

impl<'de> Visitor<'de> for ReferenceVisitor {
    type Value = Reference;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "one of `{}`", Self::FIELDS.join("`, `"))
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut local_label = None;
        let mut local_param = Vec::new();

        while let Some(key) = access.next_key::<Cow<str>>()? {
            match key.as_ref() {
                "label" => {
                    if local_label.is_some() {
                        return Err(M::Error::duplicate_field("label"));
                    }
                    local_label = Some(access.next_value()?);
                },
                "param" => {
                    let expr = access.next_value()?;
                    local_param.push(expr);
                },
                _ => {},
            }
        }

        let label = local_label.ok_or_else(|| M::Error::missing_field("label"))?;
        let params = local_param;

        Ok(Reference {
            label,
            params,
        })
    }
}

impl<'de> Deserialize<'de> for Reference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(ReferenceVisitor)
    }
}

/// A reference to a given entity.
#[derive(Debug, Clone)]
pub enum EntityRef<T> {
    /// A named entity.
    Ref(Reference),
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
    pub fn entity(&self, lookup: &dyn EntityLookup<T>) -> Result<Rc<T>, EntityError> {
        match *self {
            EntityRef::Ref(ref refer) => {
                lookup
                    .find(&refer.label)
                    .ok_or_else(|| EntityError::cannot_find(refer.label.clone()))
            },
            EntityRef::Real(ref rc) => Ok(rc.clone()),
        }
    }
}

mod private {
    use crate::data::{Action, Bullet, Fire};

    pub trait NamedEntityRef {
        const INSTANCE_NAME: &'static str;
        const REF_NAME: &'static str;
    }

    impl NamedEntityRef for Action {
        const INSTANCE_NAME: &'static str = "action";
        const REF_NAME: &'static str = "actionRef";
    }

    impl NamedEntityRef for Bullet {
        const INSTANCE_NAME: &'static str = "bullet";
        const REF_NAME: &'static str = "bulletRef";
    }

    impl NamedEntityRef for Fire {
        const INSTANCE_NAME: &'static str = "fire";
        const REF_NAME: &'static str = "fireRef";
    }
}

struct EntityRefVisitor<T> {
    marker: PhantomData<T>,
}

impl<T> EntityRefVisitor<T> {
    fn new() -> Self {
        EntityRefVisitor {
            marker: PhantomData,
        }
    }
}

impl<T> EntityRefVisitor<T>
where
    T: self::private::NamedEntityRef,
{
    const FIELDS: &'static [&'static str] = &[T::INSTANCE_NAME, T::REF_NAME];
}

impl<'de, T> Visitor<'de> for EntityRefVisitor<T>
where
    T: Deserialize<'de>,
    T: self::private::NamedEntityRef,
{
    type Value = EntityRef<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "`{}` or `{}`", T::INSTANCE_NAME, T::REF_NAME)
    }

    fn visit_enum<E>(self, access: E) -> Result<Self::Value, E::Error>
    where
        E: EnumAccess<'de>,
    {
        let (name, v): (Cow<str>, _) = access.variant()?;
        if name == T::INSTANCE_NAME {
            Ok(EntityRef::Real(Rc::new(v.newtype_variant()?)))
        } else if name == T::REF_NAME {
            let iref = v.newtype_variant::<Reference>()?;
            Ok(EntityRef::Ref(iref))
        } else {
            Err(E::Error::unknown_variant(&name, Self::FIELDS))
        }
    }
}

impl<'de, T> Deserialize<'de> for EntityRef<T>
where
    T: Deserialize<'de>,
    T: self::private::NamedEntityRef,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_enum(
            "EntityRef",
            EntityRefVisitor::<T>::FIELDS,
            EntityRefVisitor::new(),
        )
    }
}

/// Create a new bullet.
#[derive(Debug, Clone)]
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

struct FireVisitor;

impl FireVisitor {
    const FIELDS: &'static [&'static str] = &["label", "direction", "speed", "bullet", "bulletRef"];
}

impl<'de> Visitor<'de> for FireVisitor {
    type Value = Fire;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "one of `{}`", Self::FIELDS.join("`, `"))
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut local_label = None;
        let mut local_direction = None;
        let mut local_speed = None;
        let mut local_bullet = None;

        while let Some(key) = access.next_key::<Cow<str>>()? {
            match key.as_ref() {
                "label" => {
                    if local_label.is_some() {
                        return Err(M::Error::duplicate_field("label"));
                    }
                    local_label = Some(access.next_value()?);
                },
                "direction" => {
                    if local_direction.is_some() {
                        return Err(M::Error::duplicate_field("direction"));
                    }
                    local_direction = Some(access.next_value()?);
                },
                "speed" => {
                    if local_speed.is_some() {
                        return Err(M::Error::duplicate_field("speed"));
                    }
                    local_speed = Some(access.next_value()?);
                },
                "bullet" => {
                    if local_bullet.is_some() {
                        return Err(M::Error::duplicate_field("bullet or bulletRef"));
                    }
                    let bullet = access.next_value()?;
                    local_bullet = Some(EntityRef::Real(bullet));
                },
                "bulletRef" => {
                    if local_bullet.is_some() {
                        return Err(M::Error::duplicate_field("bullet or bulletRef"));
                    }
                    let iref = access.next_value::<Reference>()?;
                    local_bullet = Some(EntityRef::Ref(iref));
                },
                _ => {},
            }
        }

        let label = local_label.unwrap_or(None);
        let direction = local_direction.unwrap_or(None);
        let speed = local_speed.unwrap_or(None);
        let bullet = local_bullet.ok_or_else(|| M::Error::missing_field("bullet or bulletRef"))?;

        Ok(Fire {
            label,
            direction,
            speed,
            bullet,
        })
    }
}

impl<'de> Deserialize<'de> for Fire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(FireVisitor)
    }
}

/// Horizontal change description.
#[derive(Debug, Clone, Deserialize)]
pub struct Horizontal {
    /// How to change horizontally.
    #[serde(default, rename = "type")]
    pub kind: Change,
    /// How much to change by.
    #[serde(rename = "$value")]
    pub change: Expression,
}

/// Repetition action.
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct Repeat {
    /// How many times to repeat the actions.
    pub times: Times,
    /// The actions to repeat.
    #[serde(flatten)]
    #[serde_as(as = "EnumMap")]
    pub actions: Vec<EntityRef<Action>>,
}

/// A change in speed.
#[derive(Debug, Clone, Deserialize)]
pub struct Speed {
    /// How to change the speed.
    #[serde(default, rename = "type")]
    pub kind: Change,
    /// How much to change the speed by.
    #[serde(rename = "$value")]
    pub change: Expression,
}

/// An expression to compute a value for an action.
#[derive(Debug, Clone, Deserialize)]
pub struct Term {
    /// The value of the term.
    #[serde(rename = "$value")]
    pub value: Expression,
}

impl Term {
    /// Evaluate the term in the given context.
    pub fn eval(&self, ctx: &dyn ExpressionContext) -> Result<Value, ExpressionError> {
        self.value.eval(ctx)
    }
}

/// A count of how many times to repeat an action.
#[derive(Debug, Clone, Deserialize)]
pub struct Times {
    /// How many times to repeat an action.
    #[serde(rename = "$value")]
    pub value: Expression,
}

/// Cause the bullet to vanish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Vanish {}

/// Vertical change description.
#[derive(Debug, Clone, Deserialize)]
pub struct Vertical {
    /// How to change vertically.
    #[serde(default, rename = "type")]
    pub kind: Change,
    /// How much to change by.
    #[serde(rename = "$value")]
    pub change: Expression,
}

/// Pause execution for a given number of frames.
#[derive(Debug, Clone, Deserialize)]
pub struct Wait {
    /// The number of frames to wait for.
    #[serde(rename = "$value")]
    pub frames: Expression,
}
