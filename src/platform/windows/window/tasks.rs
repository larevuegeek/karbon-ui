//! Seul adaptateur natif du service de tâches : réveil par message, résultats sur le thread UI.
use super::{
    super::{ffi::Hwnd, present::runtime},
    state,
};
use crate::task::{TaskCommand, TaskError, TaskExecutor, TaskResult, TaskScope};
use std::cell::RefCell;
use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

// Message privé : des résultats de tâches attendent d'être livrés à cette fenêtre.
pub(crate) const TASK_WAKE: u32 = WM_APP + 2;
pub(crate) struct NativeTasks {
    scope: TaskScope,
    commands: Vec<TaskCommand>,
}
static TASKS: state::WindowLocal<RefCell<Option<NativeTasks>>> =
    state::WindowLocal::new(|state| &state.tasks);

pub(super) fn initialize(window: Hwnd) {
    let handle = window as usize;
    TASKS.with(|slot| {
        *slot.borrow_mut() = Some(NativeTasks {
            scope: TaskScope::new(move || {
                // Aucune valeur ni aucun pointeur de résultat dans le message.
                // Une boîte fermée ne publie plus. Un réveil ancien est sans données.
                // SAFETY : PostMessageW n'accède à aucune mémoire ; un handle détruit échoue.
                unsafe {
                    PostMessageW(handle as Hwnd, TASK_WAKE, 0, 0);
                }
            }),
            commands: Vec::new(),
        })
    });
}
pub(crate) fn close() {
    let tasks = TASKS.with(|slot| slot.borrow_mut().take());
    drop(tasks);
}
pub(crate) fn defer_commands() {
    if !TASKS.with(|slot| slot.borrow().is_some()) {
        return;
    }
    let commands = runtime::take_task_commands();
    if commands.is_empty() {
        return;
    }
    TASKS.with(|slot| {
        if let Some(tasks) = slot.borrow_mut().as_mut() {
            // Invalider dès la demande, avant un éventuel réveil ancien déjà en file.
            for command in &commands {
                match command {
                    TaskCommand::Spawn(task) => tasks.scope.cancel(task.id),
                    TaskCommand::Cancel(id) => tasks.scope.cancel(*id),
                    TaskCommand::CancelAll => tasks.scope.cancel_all(),
                }
            }
            tasks.commands.extend(commands);
        }
    });
    super::wake();
}
pub(crate) fn deliver(window: Hwnd) -> bool {
    let mut delivered = false;
    while runtime::can_receive_tasks() {
        let result = TASKS.with(|slot| {
            slot.borrow_mut()
                .as_mut()
                .and_then(|tasks| tasks.scope.take_next())
        });
        let Some(result) = result else {
            break;
        };
        runtime::complete_task(window, result);
        // Une réponse peut annuler une autre tâche dont le résultat attend déjà.
        defer_commands();
        delivered = true;
    }
    delivered
}

#[derive(Default)]
pub(crate) struct Tasks {
    executor: RefCell<Option<TaskExecutor>>,
}
impl Tasks {
    pub(crate) fn process(&self) -> bool {
        let mut handled = false;
        for (window, _) in state::windows() {
            let _scope = state::enter(window);
            defer_commands();
            let commands = TASKS.with(|slot| {
                slot.borrow_mut()
                    .as_mut()
                    .map_or_else(Vec::new, |tasks| std::mem::take(&mut tasks.commands))
            });
            handled |= !commands.is_empty();
            for command in commands {
                let error = match command {
                    TaskCommand::Spawn(task) => {
                        let mut executor = self.executor.borrow_mut();
                        if executor.is_none() {
                            match TaskExecutor::for_ui() {
                                Ok(pool) => *executor = Some(pool),
                                Err(error) => {
                                    let result = TaskResult::failure(
                                        task.id,
                                        TaskError::StartFailed(error.to_string()),
                                    );
                                    drop(executor);
                                    runtime::complete_task(window, result);
                                    continue;
                                }
                            }
                        }
                        TASKS.with(|slot| {
                            slot.borrow_mut().as_mut().and_then(|tasks| {
                                tasks
                                    .scope
                                    .spawn(task, executor.as_ref().expect("initialized executor"))
                            })
                        })
                    }
                    TaskCommand::Cancel(id) => {
                        TASKS.with(|slot| {
                            if let Some(tasks) = slot.borrow_mut().as_mut() {
                                tasks.scope.cancel(id);
                            }
                        });
                        None
                    }
                    TaskCommand::CancelAll => {
                        TASKS.with(|slot| {
                            if let Some(tasks) = slot.borrow_mut().as_mut() {
                                tasks.scope.cancel_all();
                            }
                        });
                        None
                    }
                };
                // Aucun emprunt du gestionnaire ni du scope pendant le callback applicatif.
                if let Some(error) = error {
                    runtime::complete_task(window, error);
                }
            }
            handled |= deliver(window);
        }
        if let Some(executor) = self.executor.borrow().as_ref() {
            executor.discard_cancelled();
        }
        handled
    }
}
