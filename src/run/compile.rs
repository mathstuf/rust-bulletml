// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

#![allow(dead_code)]

use std::collections::hash_map::HashMap;
use std::rc::Rc;

use crates::failure::Fallible;

use data::{self, EntityLookup};
pub use data::{
    Accel, Change, ChangeDirection, ChangeSpeed, Direction, DirectionKind, Expression,
    ExpressionContext, Horizontal, Orientation, Speed, Term, Times, Value, Vanish, Vertical, Wait,
};
use run::util;

/// Entities which may appear within an action.
#[derive(Debug)]
pub enum Step {
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

impl Step {
    fn new(lib: &mut Library, data_lib: &mut DataLibrary, step: &data::Step) -> Fallible<Self> {
        match *step {
            data::Step::ChangeSpeed(ref cs) => Ok(Step::ChangeSpeed(cs.clone())),
            data::Step::ChangeDirection(ref cd) => Ok(Step::ChangeDirection(cd.clone())),
            data::Step::Accel(ref accel) => Ok(Step::Accel(accel.clone())),
            data::Step::Wait(ref wait) => Ok(Step::Wait(wait.clone())),
            data::Step::Vanish(vanish) => Ok(Step::Vanish(vanish)),
            data::Step::Repeat(ref repeat) => Repeat::new(lib, data_lib, repeat).map(Step::Repeat),
            data::Step::Fire(ref fire) => {
                let entity = fire.entity(data_lib)?;
                Fire::new(lib, data_lib, entity).map(Step::Fire)
            },
            data::Step::Action(ref action) => {
                let entity = action.entity(data_lib)?;
                Action::new(lib, data_lib, entity).map(Step::Action)
            },
        }
    }
}

/// An action that may be performed for a bullet.
#[derive(Debug)]
pub struct Action {
    /// The steps which make up the action.
    pub steps: Vec<Step>,
}

impl Action {
    fn new(
        lib: &mut Library,
        data_lib: &mut DataLibrary,
        action: Rc<data::Action>,
    ) -> Fallible<Rc<Self>> {
        let comp_action = Rc::new(Action {
            steps: action
                .steps
                .iter()
                .map(|step| Step::new(lib, data_lib, step))
                .collect::<Result<Vec<_>, _>>()?,
        });

        util::ro::<_, util::EntityError>(action.label.as_ref().map(|name| {
            util::try_insert(
                name.clone(),
                &mut lib.actions,
                || comp_action.clone(),
                "action",
            )?;
            util::try_insert(
                name.clone(),
                &mut data_lib.actions,
                || action.clone(),
                "action",
            )?;
            Ok(())
        }))?;

        Ok(comp_action)
    }
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
    ) -> Fallible<Rc<Self>> {
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

        util::ro::<_, util::EntityError>(bullet.label.as_ref().map(|name| {
            util::try_insert(
                name.clone(),
                &mut lib.bullets,
                || comp_bullet.clone(),
                "bullet",
            )?;
            util::try_insert(
                name.clone(),
                &mut data_lib.bullets,
                || bullet.clone(),
                "bullet",
            )?;
            Ok(())
        }))?;

        Ok(comp_bullet)
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

/// The top-level BulletML entity.
#[derive(Debug, Clone)]
pub struct BulletML {
    /// The library of elements for the script.
    library: Library,
    /// The orientation of the game.
    pub orientation: Orientation,
    /// The actions which make up the entity.
    pub actions: Vec<Rc<Action>>,
}

impl BulletML {
    pub fn new(bulletml: data::BulletML) -> Fallible<Self> {
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
                            Err(err) => Some(Err(err)),
                        }
                    },
                    data::Element::Fire(fire) => {
                        let fire = Fire::new(&mut library, &mut data_library, fire);
                        match fire {
                            Ok(_) => None,
                            Err(err) => Some(Err(err)),
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
                            Err(err) => Some(Err(err)),
                        }
                    },
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let actions = top_actions
            .into_iter()
            .map(|action| Action::new(&mut library, &mut data_library, action))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(BulletML {
            orientation: bulletml.orientation,
            actions,
            library,
        })
    }
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
    ) -> Fallible<Rc<Self>> {
        let comp_fire = Rc::new(Fire {
            direction: fire.direction.clone(),
            speed: fire.speed.clone(),
            bullet: {
                let entity = fire.bullet.entity(data_lib)?;
                Bullet::new(lib, data_lib, entity)?
            },
        });

        util::ro::<_, util::EntityError>(fire.label.as_ref().map(|name| {
            util::try_insert(name.clone(), &mut lib.fires, || comp_fire.clone(), "fire")?;
            util::try_insert(name.clone(), &mut data_lib.fires, || fire.clone(), "fire")?;
            Ok(())
        }))?;

        Ok(comp_fire)
    }
}

/// Repetition action.
#[derive(Debug)]
pub struct Repeat {
    /// How many times to repeat the actions.
    pub times: Times,
    /// The actions to repeat.
    pub actions: Vec<Rc<Action>>,
}

impl Repeat {
    fn new(lib: &mut Library, data_lib: &mut DataLibrary, repeat: &data::Repeat) -> Fallible<Self> {
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
}

pub trait Acceleration {
    fn amount(&self, ctx: &ExpressionContext) -> Fallible<f32>;
    fn modify(&self, value: f32, current: f32, duration: f32) -> f32;
}

impl Acceleration for Horizontal {
    fn amount(&self, ctx: &ExpressionContext) -> Fallible<f32> {
        self.change.eval(ctx)
    }

    fn modify(&self, value: f32, current: f32, duration: f32) -> f32 {
        self.kind.modify(value, current, duration)
    }
}

impl Acceleration for Vertical {
    fn amount(&self, ctx: &ExpressionContext) -> Fallible<f32> {
        self.change.eval(ctx)
    }

    fn modify(&self, value: f32, current: f32, duration: f32) -> f32 {
        self.kind.modify(value, current, duration)
    }
}
