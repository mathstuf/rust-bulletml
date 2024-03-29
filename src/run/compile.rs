// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

use std::collections::hash_map::HashMap;
use std::iter;
use std::rc::Rc;

use thiserror::Error;

use crate::data::{self, EntityLookup, ExpressionError};
pub use crate::data::{
    Accel, Change, ChangeDirection, ChangeSpeed, Direction, DirectionKind, Expression,
    ExpressionContext, Horizontal, Orientation, Speed, Term, Times, Value, Vanish, Vertical, Wait,
};
use crate::run::compile;
use crate::run::util;
use crate::run::{Node, ZipperIter};

/// Entities which may appear within an action tree.
#[derive(Debug)]
pub enum NodeStep {
    Root,
    /// Cause a set of actions to be repeated a number of times.
    Repeat(Repeat),
    /// Cause a set bullets to be fired.
    Fire(Rc<Fire>),
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
}

/// Entities which may appear within an action.
#[derive(Debug, Clone)]
enum Step {
    /// Cause a set of actions to be repeated a number of times.
    Repeat(Repeat),
    /// Cause a set bullets to be fired.
    Fire(Rc<Fire>),
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
    Action(Rc<Action>),
}

#[derive(Debug, Error)]
pub enum StepError {
    #[error("lookup entity")]
    EntityLookup {
        #[from]
        source: data::EntityError,
    },
    #[error("<repeat> error")]
    Repeat {
        #[from]
        source: RepeatError,
    },
    #[error("<fire> error")]
    Fire {
        #[from]
        source: FireError,
    },
    #[error("<action> error")]
    Action {
        #[from]
        source: ActionError,
    },
}

impl Step {
    fn new(
        lib: &mut Library,
        data_lib: &mut DataLibrary,
        step: &data::Step,
    ) -> Result<Self, StepError> {
        match *step {
            data::Step::ChangeSpeed(ref cs) => Ok(Step::ChangeSpeed(cs.clone())),
            data::Step::ChangeDirection(ref cd) => Ok(Step::ChangeDirection(cd.clone())),
            data::Step::Accel(ref accel) => Ok(Step::Accel(accel.clone())),
            data::Step::Wait(ref wait) => Ok(Step::Wait(wait.clone())),
            data::Step::Vanish(vanish) => Ok(Step::Vanish(vanish)),
            data::Step::Repeat(ref repeat) => {
                Ok(Repeat::new(lib, data_lib, repeat).map(Step::Repeat)?)
            },
            data::Step::Fire(ref fire) => {
                let entity = fire.entity(data_lib)?;
                Ok(Fire::new(lib, data_lib, entity).map(Step::Fire)?)
            },
            data::Step::Action(ref action) => {
                let entity = action.entity(data_lib)?;
                Ok(Action::new(lib, data_lib, entity).map(Step::Action)?)
            },
        }
    }

    fn into_node(self) -> Node<NodeStep> {
        match self {
            Step::ChangeSpeed(cs) => Node::new(NodeStep::ChangeSpeed(cs)),
            Step::ChangeDirection(cd) => Node::new(NodeStep::ChangeDirection(cd)),
            Step::Accel(accel) => Node::new(NodeStep::Accel(accel)),
            Step::Wait(wait) => Node::new(NodeStep::Wait(wait)),
            Step::Vanish(vanish) => Node::new(NodeStep::Vanish(vanish)),
            Step::Repeat(repeat) => Node::new(NodeStep::Repeat(repeat)),
            Step::Fire(fire) => Node::new(NodeStep::Fire(fire)),
            Step::Action(action) => {
                let mut node = Node::new(NodeStep::Root);
                action
                    .steps
                    .iter()
                    .cloned()
                    .for_each(|step| node.add_child(step.into_node()));

                node
            },
        }
    }
}

/// An action that may be performed for a bullet.
#[derive(Debug)]
pub struct Action {
    /// The steps which make up the action.
    steps: Vec<Step>,
}

#[derive(Debug, Error)]
pub enum ActionError {
    #[error("lookup entity")]
    EntityLookup {
        #[from]
        source: data::EntityError,
    },
    #[error("using entity")]
    EntityUse {
        #[from]
        source: util::EntityError,
    },
    #[error("<step> error")]
    Step {
        #[from]
        source: Box<StepError>,
    },
}

impl Action {
    fn new(
        lib: &mut Library,
        data_lib: &mut DataLibrary,
        action: Rc<data::Action>,
    ) -> Result<Rc<Self>, ActionError> {
        let comp_action = Rc::new(Action {
            steps: action
                .steps
                .iter()
                .map(|step| Step::new(lib, data_lib, step))
                .collect::<Result<Vec<_>, _>>()
                .map_err(Box::new)?,
        });

        action
            .label
            .as_ref()
            .map(|name| {
                util::try_insert(
                    name.clone(),
                    &mut lib.actions,
                    || comp_action.clone(),
                    "action",
                )
                .and_then(|_| {
                    util::try_insert(
                        name.clone(),
                        &mut data_lib.actions,
                        || action.clone(),
                        "action",
                    )
                })
            })
            .transpose()?;

        Ok(comp_action)
    }

    fn node(&self) -> Node<NodeStep> {
        let mut node = Node::new(NodeStep::Root);
        self.steps
            .iter()
            .cloned()
            .for_each(|step| node.add_child(step.into_node()));

        node
    }
}

#[derive(Debug, Error)]
pub enum BulletError {
    #[error("using entity")]
    EntityUse {
        #[from]
        source: util::EntityError,
    },
    #[error("<action> error")]
    Action {
        #[from]
        source: ActionError,
    },
}

/// A bullet.
#[derive(Debug)]
pub struct Bullet {
    /// The direction to fire the bullet.
    pub direction: Option<Direction>,
    /// The initial speed of the bullet.
    pub speed: Option<Speed>,
    /// The set of actions to perform on the bullet.
    pub actions: Vec<Rc<Action>>,
}

impl Bullet {
    fn new(
        lib: &mut Library,
        data_lib: &mut DataLibrary,
        bullet: Rc<data::Bullet>,
    ) -> Result<Rc<Self>, BulletError> {
        let comp_bullet = Rc::new(Bullet {
            direction: bullet.direction.clone(),
            speed: bullet.speed.clone(),
            actions: bullet
                .actions
                .iter()
                .map(|action| {
                    let entity = action.entity(data_lib)?;
                    Action::new(lib, data_lib, entity)
                })
                .collect::<Result<Vec<_>, _>>()?,
        });

        bullet
            .label
            .as_ref()
            .map(|name| {
                util::try_insert(
                    name.clone(),
                    &mut lib.bullets,
                    || comp_bullet.clone(),
                    "bullet",
                )
                .and_then(|_| {
                    util::try_insert(
                        name.clone(),
                        &mut data_lib.bullets,
                        || bullet.clone(),
                        "bullet",
                    )
                })
            })
            .transpose()?;

        Ok(comp_bullet)
    }
}

#[derive(Debug, Clone, Default)]
struct Library {
    actions: HashMap<String, Rc<Action>>,
    bullets: HashMap<String, Rc<Bullet>>,
    fires: HashMap<String, Rc<Fire>>,
}

#[derive(Debug, Clone, Default)]
struct DataLibrary {
    actions: HashMap<String, Rc<data::Action>>,
    bullets: HashMap<String, Rc<data::Bullet>>,
    fires: HashMap<String, Rc<data::Fire>>,
}

impl EntityLookup<data::Action> for DataLibrary {
    fn find(&self, name: &str) -> Option<Rc<data::Action>> {
        self.actions.get(name).map(Clone::clone)
    }
}

impl EntityLookup<data::Bullet> for DataLibrary {
    fn find(&self, name: &str) -> Option<Rc<data::Bullet>> {
        self.bullets.get(name).map(Clone::clone)
    }
}

impl EntityLookup<data::Fire> for DataLibrary {
    fn find(&self, name: &str) -> Option<Rc<data::Fire>> {
        self.fires.get(name).map(Clone::clone)
    }
}

#[derive(Debug, Error)]
pub enum BulletMLError {
    #[error("<action> error")]
    Action {
        #[from]
        source: compile::ActionError,
    },
    #[error("<bullet> error")]
    Bullet {
        #[from]
        source: compile::BulletError,
    },
    #[error("<fire> error")]
    Fire {
        #[from]
        source: compile::FireError,
    },
}

/// The top-level BulletML entity.
#[derive(Debug)]
pub struct BulletML {
    /// The orientation of the game.
    pub orientation: Orientation,
    /// The actions which make up the entity.
    pub steps: ZipperIter<NodeStep>,
}

impl BulletML {
    pub fn new(bulletml: data::BulletML) -> Result<Self, BulletMLError> {
        let mut library = Library::default();
        let mut data_library = DataLibrary::default();

        let top_actions = bulletml
            .elements
            .into_iter()
            .filter_map(|element| {
                match element {
                    data::Element::Bullet(bullet) => {
                        let bullet = Bullet::new(&mut library, &mut data_library, bullet);
                        match bullet {
                            Ok(_) => None,
                            Err(err) => Some(Err(err.into())),
                        }
                    },
                    data::Element::Fire(fire) => {
                        let fire = Fire::new(&mut library, &mut data_library, fire);
                        match fire {
                            Ok(_) => None,
                            Err(err) => Some(Err(err.into())),
                        }
                    },
                    data::Element::Action(action) => {
                        if let Some(label) = action.label.clone() {
                            if label.starts_with("top") {
                                return Some(Ok(action));
                            }
                        }

                        let action = Action::new(&mut library, &mut data_library, action);
                        match action {
                            Ok(_) => None,
                            Err(err) => Some(Err(err.into())),
                        }
                    },
                }
            })
            .collect::<Result<Vec<_>, BulletMLError>>()?;
        let actions = top_actions
            .into_iter()
            .map(|action| Action::new(&mut library, &mut data_library, action))
            .collect::<Result<Vec<_>, _>>()?;
        let mut node = Node::new(NodeStep::Root);
        actions
            .into_iter()
            .for_each(|action| node.add_child(action.node()));

        Ok(BulletML {
            orientation: bulletml.orientation,
            steps: node.zipper().iter(),
        })
    }
}

#[derive(Debug, Error)]
pub enum FireError {
    #[error("lookup entity")]
    EntityLookup {
        #[from]
        source: data::EntityError,
    },
    #[error("using entity")]
    EntityUse {
        #[from]
        source: util::EntityError,
    },
    #[error("<bullet> error")]
    Bullet {
        #[from]
        source: compile::BulletError,
    },
}

/// Create a new bullet.
#[derive(Debug)]
pub struct Fire {
    /// The direction to fire in.
    pub direction: Option<Direction>,
    /// The initial speed of the bullet.
    pub speed: Option<Speed>,
    /// The bullet to fire.
    pub bullet: Rc<Bullet>,
}

impl Fire {
    fn new(
        lib: &mut Library,
        data_lib: &mut DataLibrary,
        fire: Rc<data::Fire>,
    ) -> Result<Rc<Self>, FireError> {
        let comp_fire = Rc::new(Fire {
            direction: fire.direction.clone(),
            speed: fire.speed.clone(),
            bullet: {
                let entity = fire.bullet.entity(data_lib)?;
                Bullet::new(lib, data_lib, entity)?
            },
        });

        fire.label
            .as_ref()
            .map(|name| {
                util::try_insert(name.clone(), &mut lib.fires, || comp_fire.clone(), "fire")
                    .and_then(|_| {
                        util::try_insert(name.clone(), &mut data_lib.fires, || fire.clone(), "fire")
                    })
            })
            .transpose()?;

        Ok(comp_fire)
    }
}

#[derive(Debug, Error)]
pub enum RepeatError {
    #[error("<action> error")]
    Action {
        #[from]
        source: ActionError,
    },
}

/// Repetition action.
#[derive(Debug, Clone)]
pub struct Repeat {
    /// How many times to repeat the actions.
    pub times: Times,
    /// The actions to repeat.
    actions: Vec<Rc<Action>>,
}

impl Repeat {
    fn new(
        lib: &mut Library,
        data_lib: &mut DataLibrary,
        repeat: &data::Repeat,
    ) -> Result<Self, RepeatError> {
        Ok(Repeat {
            times: repeat.times.clone(),
            actions: repeat
                .actions
                .iter()
                .map(|action| {
                    let entity = action.entity(data_lib)?;
                    Action::new(lib, data_lib, entity)
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub fn new_steps(&self, count: usize) -> Vec<Node<NodeStep>> {
        iter::repeat(())
            .take(count)
            .map(|_| self.actions.iter().cloned())
            .flatten()
            .map(|action| Step::Action(action).into_node())
            .collect()
    }
}

pub trait Acceleration {
    fn amount(&self, ctx: &dyn ExpressionContext) -> Result<f32, ExpressionError>;
    fn modify(&self, value: f32, current: f32, duration: f32) -> f32;
}

impl Acceleration for Horizontal {
    fn amount(&self, ctx: &dyn ExpressionContext) -> Result<f32, ExpressionError> {
        self.change.eval(ctx)
    }

    fn modify(&self, value: f32, current: f32, duration: f32) -> f32 {
        self.kind.modify(value, current, duration)
    }
}

impl Acceleration for Vertical {
    fn amount(&self, ctx: &dyn ExpressionContext) -> Result<f32, ExpressionError> {
        self.change.eval(ctx)
    }

    fn modify(&self, value: f32, current: f32, duration: f32) -> f32 {
        self.kind.modify(value, current, duration)
    }
}
