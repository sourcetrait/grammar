use crate::*;

/// Max concurrent in-process evals: available parallelism halved, min 1.
pub fn eval_concurrency_cap() -> usize {
    std::thread::available_parallelism()
        .map(|n| (n.get() / 2).max(1))
        .unwrap_or(1)
}

/// The stateless eval executor: a swappable base plus a buffer of ready clones.
pub struct Executor {
    base: std::sync::Mutex<BaseHolder>,
    ready: std::sync::Mutex<Vec<ReadyClone>>,
    env_jobs: Arc<std::sync::Mutex<nu::Jobs>>,
    semaphore: Arc<tk::Semaphore>,
    ready_target: usize,
}

struct BaseHolder {
    engine: Arc<nu::EngineState>,
    generation: u64,
    registry_mtime: Option<SystemTime>,
}

struct ReadyClone {
    engine: nu::EngineState,
    generation: u64,
}

impl Executor {
    /// Build the base once, snapshot the registry mtime, pre-fill the buffer.
    pub fn new(env_jobs: Arc<std::sync::Mutex<nu::Jobs>>, ready_target: usize) -> Self {
        let holder = BaseHolder {
            engine: Arc::new(startup(Mode::Stateless)),
            generation: 0,
            registry_mtime: registry_mtime(),
        };
        let executor = Self {
            base: std::sync::Mutex::new(holder),
            ready: std::sync::Mutex::new(Vec::new()),
            env_jobs,
            semaphore: Arc::new(tk::Semaphore::new(eval_concurrency_cap())),
            ready_target,
        };
        executor.refill_ready();
        executor
    }

    /// The concurrency gate: acquire a permit before launching an eval.
    pub fn semaphore(&self) -> Arc<tk::Semaphore> {
        self.semaphore.clone()
    }

    /// The base's lock; a poisoned one recovers with the base rebuilt.
    fn base(&self) -> std::sync::MutexGuard<'_, BaseHolder> {
        match self.base.lock() {
            Ok(holder) => holder,
            Err(poisoned) => {
                let mut holder = poisoned.into_inner();
                holder.engine = Arc::new(startup(Mode::Stateless));
                holder.generation += 1;
                holder.registry_mtime = registry_mtime();
                self.base.clear_poison();
                holder
            }
        }
    }

    /// The ready buffer's lock; a poisoned one recovers with the buffer emptied.
    fn ready(&self) -> std::sync::MutexGuard<'_, Vec<ReadyClone>> {
        match self.ready.lock() {
            Ok(ready) => ready,
            Err(poisoned) => {
                let mut ready = poisoned.into_inner();
                ready.clear();
                self.ready.clear_poison();
                ready
            }
        }
    }

    /// Top the ready buffer up to `ready_target` with current-generation clones.
    fn refill_ready(&self) {
        loop {
            let need = {
                let ready = self.ready();
                self.ready_target.saturating_sub(ready.len())
            };
            if need == 0 {
                break;
            }
            let (engine, generation) = {
                let holder = self.base();
                let mut engine = (*holder.engine).clone();
                engine.jobs = self.env_jobs.clone();
                (engine, holder.generation)
            };
            let mut ready = self.ready();
            if ready.len() < self.ready_target {
                ready.push(ReadyClone { engine, generation });
            } else {
                break;
            }
        }
    }

    /// Rebuild the base if the plugin registry moved since it was built.
    pub fn refresh_base_if_stale(&self) {
        let current = registry_mtime();
        {
            let mut holder = self.base();
            if current == holder.registry_mtime {
                return;
            }
            holder.engine = Arc::new(startup(Mode::Stateless));
            holder.generation += 1;
            holder.registry_mtime = current;
        }
        self.ready().clear();
        self.refill_ready();
    }

    /// Take a ready clone, or clone fresh, then refill for the next caller.
    pub fn take_clone(&self) -> nu::EngineState {
        let engine = {
            let holder = self.base();
            let generation = holder.generation;
            let mut ready = self.ready();
            ready.retain(|rc| rc.generation == generation);
            match ready.pop() {
                Some(rc) => rc.engine,
                None => {
                    let mut engine = (*holder.engine).clone();
                    engine.jobs = self.env_jobs.clone();
                    engine
                }
            }
        };
        self.refill_ready();
        engine
    }
}
