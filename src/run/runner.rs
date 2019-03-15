// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

use crates::failure::Fallible;

use data;
use run::compile::*;
use run::BulletManager;
use run::Node;

#[derive(Debug, Clone, Copy)]
struct Function {
    min: u32,
    max: u32,

    start: f32,
    end: f32,
    step: f32,
}

impl Function {
    fn new(min: u32, max: u32, start: f32, end: f32) -> Self {
        Function {
            min,
            max,
            start,
            end,
            step: (end - start) / ((max - min) as f32),
        }
    }

    fn call(&self, x: u32) -> f32 {
        self.start + self.step * ((x - self.min) as f32)
    }

    fn is_in_domain(&self, x: u32) -> bool {
        self.min <= x && x < self.max
    }

    fn last(&self) -> f32 {
        self.end
    }
}

enum Status {
    /// The action has completed.
    End,
    /// The action has completed; move to the next step.
    Continue,
    /// New actions should be performed.
    NewSteps(Vec<Node<NodeStep>>),
}

struct State<T> {
    manager: T,
    orientation: Orientation,

    prev_dir: Option<f32>,
    change_dir: Option<Function>,

    prev_speed: Option<f32>,
    change_speed: Option<Function>,

    accel_x: Option<Function>,
    accel_y: Option<Function>,

    next: Option<u32>,
}

macro_rules! run_function {
    ( $opt_func:expr, $turn:expr, $cb:expr ) => {
        if $opt_func.is_some() {
            let (cont, v) = {
                let func = $opt_func.as_ref().expect("checked above");
                Self::update_function(func, $turn)
            };
            $cb(v);
            if !cont {
                $opt_func = None;
            }
            true
        } else {
            false
        }
    };
}

impl<T> State<T> {
    fn new(manager: T, orientation: Orientation) -> Self {
        Self {
            manager,
            orientation,

            prev_dir: None,
            change_dir: None,

            prev_speed: None,
            change_speed: None,

            accel_x: None,
            accel_y: None,

            next: None,
        }
    }

    fn update_function(f: &Function, turn: u32) -> (bool, f32) {
        if f.is_in_domain(turn) {
            (true, f.call(turn))
        } else {
            (false, f.last())
        }
    }
}

impl<T> State<T>
where
    T: BulletManager,
{
    fn update_functions(&mut self) -> bool {
        let turn = self.manager.turn();

        let dir_updated = run_function!(self.change_dir, turn, |v| {
            self.manager.change_direction(v)
        });
        let speed_updated = run_function!(self.change_speed, turn, |v| {
            self.manager.change_speed(v)
        });
        let accel_x_updated = run_function!(self.accel_x, turn, |v| self.manager.accel_x(v));
        let accel_y_updated = run_function!(self.accel_y, turn, |v| self.manager.accel_y(v));

        dir_updated || speed_updated || accel_x_updated || accel_y_updated
    }

    fn speed_func<A>(
        &self,
        accel: Option<&A>,
        init_speed: f32,
        turn: u32,
        duration: f32,
    ) -> Fallible<Option<Function>>
    where
        A: Acceleration,
    {
        accel
            .map(|accel| {
                let change = accel.amount(&self.manager)?;
                let final_speed = accel.modify(change, init_speed, duration);
                Ok(Function::new(
                    turn,
                    turn + (duration.ceil() as u32),
                    init_speed,
                    final_speed,
                ))
            })
            .transpose()
    }

    fn run_accel(&mut self, accel: &Accel) -> Fallible<Status> {
        let duration = accel.duration.eval(&self.manager)?.max(0.);
        let turn = self.manager.turn();

        if let Orientation::Horizontal = self.orientation {
            self.accel_x = self.speed_func(
                accel.vertical.as_ref(),
                self.manager.speed_x(),
                turn,
                duration,
            )?;
            self.accel_y = self.speed_func(
                accel.horizontal.as_ref(),
                self.manager.speed_y(),
                turn,
                duration,
            )?;
        } else {
            self.accel_x = self.speed_func(
                accel.horizontal.as_ref(),
                self.manager.speed_x(),
                turn,
                duration,
            )?;
            self.accel_y = self.speed_func(
                accel.vertical.as_ref(),
                self.manager.speed_y(),
                turn,
                duration,
            )?;
        };

        Ok(Status::Continue)
    }

    fn target_direction(&self, kind: DirectionKind, degrees: f32) -> f32 {
        let dir = match kind {
            DirectionKind::Aim => {
                // Aim at the player.
                degrees + self.manager.aim_direction()
            },
            DirectionKind::Absolute => {
                // Orient according to the setup.
                self.orientation.up(degrees)
            },
            DirectionKind::Relative => {
                // Modify relative to the current direction.
                degrees + self.manager.direction()
            },
            DirectionKind::Sequence => {
                if let Some(prev_dir) = self.prev_dir {
                    // Change relative to the previous direction.
                    degrees + prev_dir
                } else {
                    // Default towards the target.
                    self.manager.aim_direction()
                }
            },
        };

        dir % 360.
    }

    fn target_direction_data(&self, direction: &Direction) -> Fallible<f32> {
        direction
            .degrees
            .eval(&self.manager)
            .map(|degrees| self.target_direction(direction.kind, degrees))
    }

    fn run_change_direction(&mut self, cd: &ChangeDirection) -> Fallible<Status> {
        let duration = cd.value.eval(&self.manager)?.max(0.);
        let direction = &cd.direction;
        let cur_dir = self.manager.direction();
        let degrees = direction.degrees.eval(&self.manager)?;

        let final_dir = if let DirectionKind::Sequence = direction.kind {
            duration * degrees + cur_dir
        } else {
            self.target_direction(direction.kind, degrees)
        };

        let turn = self.manager.turn();
        self.change_dir = Some(Function::new(
            turn,
            turn + (duration.ceil() as u32),
            cur_dir,
            final_dir,
        ));

        Ok(Status::Continue)
    }

    fn target_speed(&self, kind: Change, value: f32) -> f32 {
        match kind {
            Change::Absolute => value,
            Change::Relative => value + self.manager.speed(),
            Change::Sequence => {
                if let Some(prev_speed) = self.prev_speed {
                    value + prev_speed
                } else {
                    1.
                }
            },
        }
    }

    fn target_speed_data(&self, speed: &Speed) -> Fallible<f32> {
        speed
            .change
            .eval(&self.manager)
            .map(|change| self.target_speed(speed.kind, change))
    }

    fn run_change_speed(&mut self, cs: &ChangeSpeed) -> Fallible<Status> {
        let duration = cs.value.eval(&self.manager)?.max(0.);
        let speed = &cs.speed;
        let cur_speed = self.manager.speed();
        let change = speed.change.eval(&self.manager)?;

        let final_speed = if let Change::Sequence = speed.kind {
            duration * change + cur_speed
        } else {
            self.target_speed(speed.kind, change)
        };

        let turn = self.manager.turn();
        self.change_speed = Some(Function::new(
            turn,
            turn + (duration.ceil() as u32),
            cur_speed,
            final_speed,
        ));

        Ok(Status::Continue)
    }

    fn run_fire(&mut self, fire: &Fire) -> Fallible<Status> {
        let fire_dir = fire
            .direction
            .as_ref()
            .map(|direction| self.target_direction_data(direction))
            .transpose()?;
        let fire_speed = fire
            .speed
            .as_ref()
            .map(|speed| self.target_speed_data(speed))
            .transpose()?;

        let bullet = fire.bullet.as_ref();

        let dir = bullet
            .direction
            .as_ref()
            .map(|direction| self.target_direction_data(direction))
            .transpose()?
            .or(fire_dir)
            .unwrap_or_else(|| self.manager.aim_direction());
        let speed = bullet
            .speed
            .as_ref()
            .map(|speed| self.target_speed_data(speed))
            .transpose()?
            .or(fire_speed)
            .unwrap_or_else(|| self.manager.default_speed());

        self.prev_dir = Some(dir);
        self.prev_speed = Some(speed);

        if bullet.actions.is_empty() {
            self.manager.new_simple(dir, speed);
        } else {
            // TODO(#4): The actions need to be handled here.
            self.manager.new(dir, speed);
        }

        Ok(Status::Continue)
    }

    fn run_repeat(&mut self, repeat: &Repeat) -> Fallible<Status> {
        let times = repeat.times.value.eval(&self.manager)?;

        // Other implementations use C++'s static_cast which truncates, so compare with `1`
        // rather than letting rounding occur.
        let count = if times.is_nan() || times < 1. {
            0
        } else {
            times as usize
        };

        Ok(Status::NewSteps(repeat.new_steps(count)))
    }

    fn run_vanish(&mut self) -> Status {
        self.manager.vanish();
        Status::End
    }

    fn run_wait(&mut self, wait: &Wait) -> Fallible<Status> {
        let next = if let Some(next) = self.next {
            next
        } else {
            let frames = wait.frames.eval(&self.manager)?;
            self.manager.turn() + (frames.ceil() as u32)
        };

        Ok(if next < self.manager.turn() {
            self.next = Some(next);
            Status::End
        } else {
            self.next = None;
            Status::Continue
        })
    }
}

/// Run a script with a given bullet manager.
pub struct Runner<T> {
    state: State<T>,
    bulletml: BulletML,
}

impl<T> Runner<T> {
    /// Create a new runner for a manager and BulletML script.
    pub fn new(manager: T, bulletml: data::BulletML) -> Fallible<Self> {
        Ok(Runner {
            state: State::new(manager, bulletml.orientation),
            bulletml: BulletML::new(bulletml)?,
        })
    }
}

impl<T> Runner<T>
where
    T: BulletManager,
{
    /// Update the state.
    pub fn update(&mut self) -> Fallible<bool> {
        let mut updated = self.state.update_functions();

        loop {
            let status = {
                let mut node = if let Some(node) = self.bulletml.steps.current_mut() {
                    updated = true;
                    node
                } else {
                    break;
                };

                let status = match node.as_ref() {
                    NodeStep::Root => Status::Continue,
                    NodeStep::Repeat(ref r) => self.state.run_repeat(r)?,
                    NodeStep::Fire(ref f) => self.state.run_fire(f)?,
                    NodeStep::ChangeSpeed(ref cs) => self.state.run_change_speed(cs)?,
                    NodeStep::ChangeDirection(ref cd) => self.state.run_change_direction(cd)?,
                    NodeStep::Accel(ref a) => self.state.run_accel(a)?,
                    NodeStep::Wait(ref w) => self.state.run_wait(w)?,
                    NodeStep::Vanish(_) => self.state.run_vanish(),
                };

                if let Status::NewSteps(steps) = status {
                    steps.into_iter().for_each(|step| node.add_child(step));
                    Status::Continue
                } else {
                    status
                }
            };

            match status {
                Status::End => break,
                Status::Continue => {
                    self.bulletml.steps.next();
                },
                Status::NewSteps(_) => unreachable!(),
            }
        }

        Ok(updated)
    }
}

/*
public BulletMLRunner createRunner(BulletManager manager, BulletML bml) {
  return createRunner(manager, resolve(bml));
}

public BulletMLRunner createRunner(BulletManager manager, const ResolvedBulletML bml) {
  return new GroupRunner(manager, bml);
}

public interface BulletMLRunner {
  private:
    public bool done();
    public void run();
}

private class GroupRunner: BulletMLRunner {
  private:
    BulletMLRunner[] runners;

    package this(BulletManager manager, const ResolvedBulletML bml) {
      BulletML.Orientation orientation = bml.get().orientation;
      foreach (elem; bml.get().elements) {
        elem.tryVisit!(
          (Action action) {
            runners ~= new ActionRunner(manager, orientation, action);
          },
          () {
          })();
      }
    }

    public bool done() {
      foreach (runner; runners) {
        if (!runner.done()) {
          return false;
        }
      }

      return true;
    }

    public void run() {
      foreach (runner; runners) {
        runner.run();
      }
    }
}

public class ActionRunner: BulletMLRunner {
  private:
    private class ActionZipper {
      public:
        ActionZipper par;
        Action.AElement[] actions;
      private:
        size_t idx;
        size_t repeat;

        public this(ActionZipper parent, Action.AElement[] actions, size_t repeat = 1) {
          par = parent;
          this.actions = actions;
          idx = 0;
          this.repeat = repeat;
        }

        public this(Action.AElement[] actions) {
          this(null, actions);
        }

        public ActionZipper parent() {
          return par;
        }

        public bool done() {
          return actions.length == idx;
        }

        public Action.AElement current() {
          return actions[idx];
        }

        public void next() {
          ++idx;
          if (done() && --repeat) {
            idx = 0;
          }
        }
    }

    private enum Status {
      // End processing for this step.
      END,
      // Process the next node.
      CONTINUE,
      // The next action has been loaded.
      UPDATED
    }

    BulletManager manager;
    BulletML.Orientation orientation;
    ActionZipper zipper;

    Array!uint repeatStack;
    Nullable!uint next;
    Nullable!float prevSpeed;
    Nullable!float prevDirection;
    bool end;

    alias LinearFunction!(uint, double) UpdateFunction;
    alias Nullable!UpdateFunction NUpdateFunction;
    NUpdateFunction changeDirF;
    NUpdateFunction changeSpeedF;
    NUpdateFunction accelXF;
    NUpdateFunction accelYF;

    package this(BulletManager manager, BulletML.Orientation orientation, Action act) {
      this.manager = manager;
      this.orientation = orientation;
      zipper = new ActionZipper(act.contents);
      end = false;
    }

    public bool done() {
      return end;
    }

    public void run() {
      uint turn = manager.getTurn();
      bool updated = update(turn);

      // Check to see if we're at the end of the run.
      if (zipper.done()) {
        // Try to go up a level.
        if (zipper.parent() is null) {
          // We're waiting for a trailing 'wait' element.
          if (next.isNull || turn <= next.get()) {
            // End the bullet if we have no left over update functions
            // remaining.
            if (!updated) {
              end = true;
            }
          }

          return;
        }

        nextSibling(zipper);
      }

      while (!zipper.done()) {
        Status status = runAction(zipper.current(), turn);

        if (status == Status.END) {
          break;
        } else if (status == Status.CONTINUE) {
          nextAction(zipper);
        } else if (status == Status.UPDATED) {
          // zipper points to our next task already.
        }
      }
    }

    private bool update(uint turn) {
      bool updated = false;

      if (!changeDirF.isNull()) {
        double degree = updateTurn(changeDirF, turn);
        manager.changeDirection(degree);
        updated = true;
      }

      if (!changeSpeedF.isNull()) {
        double ds = updateTurn(changeSpeedF, turn);
        manager.changeSpeed(ds);
        updated = true;
      }

      if (!accelXF.isNull()) {
        double dvx = updateTurn(accelXF, turn);
        manager.accelX(dvx);
        updated = true;
      }

      if (!accelYF.isNull()) {
        double dvy = updateTurn(accelYF, turn);
        manager.accelY(dvy);
        updated = true;
      }

      return updated;
    }

    private double updateTurn(ref NUpdateFunction func, uint turn) {
      double value;

      if (func.inDomain(turn)) {
        value = func(turn);
      } else {
        value = func.last();
        func.nullify();
      }

      return value;
    }

    private void nextAction(ref ActionZipper zipper) {
      zipper.next();

      if (zipper.done() && zipper.parent() !is null) {
        nextSibling(zipper);
      }
    }

    private void nextSibling(ref ActionZipper zipper) {
      zipper = zipper.parent();
      zipper.next();
    }

    private Status runAction(Action.AElement action, uint turn) {
      return action.tryVisit!(
        (Repeat* repeat) =>
          runRepeat(*repeat, turn),
        (Fire* fire) =>
          runFire(*fire, turn),
        (ORef!Fire ofire) =>
          runFire(*ofire.target, turn),
        (ChangeSpeed changeSpeed) =>
          runChangeSpeed(changeSpeed, turn),
        (ChangeDirection changeDirection) =>
          runChangeDirection(changeDirection, turn),
        (Accel accel) =>
          runAccel(accel, turn),
        (Wait wait) =>
          runWait(wait, turn),
        (Vanish vanish) =>
          runVanish(vanish, turn),
        (Action* action) =>
          runAction(*action, turn),
        (ORef!Action oaction) =>
          runAction(*oaction.target, turn)
        )();
    }

    private Status runAction(Action action, uint turn) {
      zipper = new ActionZipper(zipper, action.contents);
      return Status.UPDATED;
    }
}
*/
