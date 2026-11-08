//! Session de la boucle : ouverture des fenêtres, commandes applicatives, tâches et
//! file de messages commune du thread UI.
use super::{
    super::{
        accessibility,
        ffi::{Hwnd, last_error},
        graphics::Renderer,
        imaging::DrawingEngine,
        present::{self, runtime},
    },
    WINDOW_WAKE, create, dwm, startup, state, take_error, take_pending, tasks,
};
use crate::window::{ExitPolicy, WindowCommand, WindowDescriptor};
use std::ptr::null_mut;
use windows_sys::Win32::{
    Graphics::Gdi::UpdateWindow,
    UI::WindowsAndMessaging::{
        DestroyWindow, DispatchMessageW, GetMessageW, MSG, SW_SHOW, SW_SHOWNOACTIVATE, ShowWindow,
        TranslateMessage,
    },
};

pub(crate) struct Session {
    decoder: DrawingEngine,
    pub(crate) tasks: tasks::Tasks,
}
impl Drop for Session {
    fn drop(&mut self) {
        // Les ressources des fenêtres doivent disparaître avant le décodeur et COM.
        for (window, _) in state::windows() {
            // SAFETY : fenêtre encore enregistrée, détruite sur son thread propriétaire.
            unsafe {
                DestroyWindow(window);
            }
        }
        drop(take_pending());
    }
}
impl Session {
    pub(crate) fn new() -> crate::Result<Self> {
        Ok(Self {
            decoder: DrawingEngine::initialize()?,
            tasks: tasks::Tasks::default(),
        })
    }
    pub(crate) fn open(&self, descriptor: WindowDescriptor, visible: bool) -> crate::Result<Hwnd> {
        if state::windows().iter().any(|(_, id)| *id == descriptor.id) {
            return Err(crate::Error::DuplicateWindowId(descriptor.id));
        }
        let WindowDescriptor {
            id,
            options,
            application,
        } = descriptor;
        if let Some(reason) = crate::window::validation::invalid_options(&options) {
            return Err(crate::Error::invalid_window_options(reason));
        }
        let state = state::WindowState::new(id, options);
        // CreateWindowEx appelle déjà window_proc : les premiers messages utilisent ce scope.
        let _scope = state::Scope::new(std::rc::Rc::clone(&state));
        // Le périphérique D3D11 se crée en parallèle du chargement de l'application et
        // de la première mise en page (voir `present::load`).
        let mut renderer = Renderer::start(options.renderer)?;
        let runtime = present::load(&mut renderer, &self.decoder, application)?;
        renderer.ready()?;
        startup::mark("renderer ready (D3D11/D2D)");
        let (window, presenter) = create::create_presented_window(options, &mut renderer)?;
        startup::mark("window and presentation created");
        state::register(window, state);
        tasks::initialize(window);
        let initialize = || -> crate::Result<()> {
            present::initialize(options.renderer, (renderer, presenter), runtime, id)?;
            accessibility::initialize(window)?;
            if !options.transparent {
                dwm::apply_style(window);
            }
            startup::mark("accessibility and window style");
            present::paint::paint_window(window);
            if let Some(error) = take_error() {
                return Err(error);
            }
            if visible {
                // SAFETY : fenêtre créée ci-dessus sur ce thread ; aucun pointeur transmis.
                unsafe {
                    ShowWindow(
                        window,
                        if options.activate {
                            SW_SHOW
                        } else {
                            SW_SHOWNOACTIVATE
                        },
                    );
                    UpdateWindow(window);
                }
                startup::mark("window shown");
            }
            super::change::notify(window);
            Ok(())
        };
        if let Err(error) = initialize() {
            // Le rendu peut avoir déjà détruit la fenêtre en signalant son erreur.
            if state::windows().iter().any(|(handle, _)| *handle == window) {
                // SAFETY : la fenêtre est encore enregistrée, donc vivante sur ce thread.
                unsafe {
                    DestroyWindow(window);
                }
            }
            return Err(error);
        }
        Ok(window)
    }

    pub(crate) fn commands(&self, visible: bool) -> crate::Result<bool> {
        let mut commands = take_pending();
        for (window, _) in state::windows() {
            let _scope = state::enter(window);
            commands.extend(runtime::take_window_commands());
        }
        let handled = !commands.is_empty();
        // Tous les emprunts d'application sont terminés avant les appels Windows.
        for command in commands {
            match command {
                WindowCommand::Open(window) => {
                    self.open(window, visible)?;
                }
                WindowCommand::Action(id, action) => {
                    if let Some((window, _)) = state::windows()
                        .into_iter()
                        .find(|(_, candidate)| *candidate == id)
                    {
                        let _scope = state::enter(window);
                        super::actions::execute_action(window, action);
                    }
                }
                WindowCommand::Change(id, change) => {
                    if let Some((window, _)) = state::windows()
                        .into_iter()
                        .find(|(_, candidate)| *candidate == id)
                    {
                        let _scope = state::enter(window);
                        super::change::execute(window, change)?;
                    }
                }
                WindowCommand::Close(id) => {
                    if let Some((window, _)) = state::windows()
                        .into_iter()
                        .find(|(_, candidate)| *candidate == id)
                    {
                        // SAFETY : fenêtre enregistrée, détruite sur son thread propriétaire.
                        unsafe {
                            DestroyWindow(window);
                        }
                    }
                }
                WindowCommand::FileDialog(id, task, dialog) => {
                    if let Some((window, _)) = state::windows()
                        .into_iter()
                        .find(|(_, candidate)| *candidate == id)
                    {
                        let result = match super::dialog::show(window, &dialog) {
                            Ok(paths) => crate::task::TaskResult::success(
                                task,
                                crate::window::FileDialogResult { paths },
                            ),
                            Err(error) => crate::task::TaskResult::failure(
                                task,
                                crate::task::TaskError::StartFailed(error.to_string()),
                            ),
                        };
                        let _scope = state::enter(window);
                        runtime::complete_task(window, result);
                        tasks::defer_commands();
                    }
                }
                WindowCommand::Exit => {
                    for (window, _) in state::windows() {
                        // SAFETY : voir ci-dessus.
                        unsafe {
                            DestroyWindow(window);
                        }
                    }
                    break;
                }
            }
        }
        Ok(handled)
    }

    pub(crate) fn should_exit(&self, policy: ExitPolicy) -> bool {
        let windows = state::windows();
        windows.is_empty()
            || matches!(policy, ExitPolicy::MainWindowClosed(id) if !windows.iter().any(|(_, candidate)| *candidate == id))
    }

    pub(crate) fn run_message_loop(&self, policy: ExitPolicy) -> crate::Result<()> {
        let mut message = MSG::default();
        loop {
            if let Some(error) = take_error() {
                return Err(error);
            }
            if self.should_exit(policy) {
                return Ok(());
            }
            if self.commands(true)? {
                continue;
            }
            if self.tasks.process() {
                continue;
            }
            // Une seule file d'événements pour toutes les fenêtres du thread UI.
            // SAFETY : `message` est une structure locale valide en écriture.
            let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
            if result == -1 {
                return Err(last_error("GetMessageW"));
            }
            if result == 0 {
                return Ok(());
            }
            if message.hwnd.is_null() && message.message == WINDOW_WAKE {
                continue;
            }
            // SAFETY : message tel que rempli par GetMessageW, relayé sur le même thread.
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}
