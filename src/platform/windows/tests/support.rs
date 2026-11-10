pub(super) use super::super::{
    ffi::{ComApartment, Hwnd},
    window::Session,
};
pub(super) use crate::{
    application::{Application, Resources, ViewContext},
    ui::{element::Element, widgets},
    window::{RenderMode, WindowCommand, WindowDescriptor, WindowId, WindowOptions},
};
pub(super) use std::{
    cell::{Cell, RefCell},
    ptr::null_mut,
    rc::Rc,
};
pub(super) use windows_sys::Win32::{
    System::Threading::GetCurrentThreadId,
    UI::{
        Input::KeyboardAndMouse::VK_F5,
        WindowsAndMessaging::{PostThreadMessageW, SendMessageW, WM_KEYDOWN, WM_PAINT, WM_QUIT},
    },
};

/// Message des applications de test : identifiant du bouton activé.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Msg {
    Click(u64),
}

pub(super) struct TestApp {
    pub(super) count: Rc<Cell<u64>>,
    pub(super) drops: Rc<Cell<u64>>,
    pub(super) commands: Rc<RefCell<Vec<WindowCommand>>>,
    pub(super) nested_target: Rc<Cell<Hwnd>>,
    pub(super) fail_load: bool,
    pub(super) exit_on_update: bool,
    pub(super) task_commands: Rc<RefCell<Vec<crate::task::TaskCommand>>>,
    pub(super) completed: Rc<RefCell<Vec<(crate::task::TaskId, u32, std::thread::ThreadId)>>>,
    pub(super) exit_on_completion: bool,
}
impl Drop for TestApp {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}
impl Application for TestApp {
    type Message = Msg;
    fn load(&mut self, _: &dyn Resources) -> crate::Result<()> {
        if self.fail_load {
            Err(crate::Error::application("load rejected by the test"))
        } else {
            Ok(())
        }
    }
    fn view(&self, _: &ViewContext) -> Element {
        Element::column(
            1,
            vec![
                widgets::text(2, format!("Compteur {}", self.count.get())),
                widgets::button(3, "Incrémenter", Msg::Click(1)),
            ],
        )
    }
    fn update(&mut self, message: Msg) -> bool {
        assert_eq!(message, Msg::Click(1));
        self.count.set(self.count.get() + 1);
        if self.exit_on_update {
            self.commands.borrow_mut().push(WindowCommand::Exit);
            return false; // Les commandes ne doivent pas dépendre d'une reconstruction.
        }
        let other = self.nested_target.get();
        if !other.is_null() {
            // Un callback B entre pendant que le runtime A est encore emprunté.
            // SAFETY : fenêtre de test de ce thread ; aucun pointeur dans wparam/lparam.
            unsafe {
                SendMessageW(other, WM_KEYDOWN, VK_F5 as usize, 0);
                SendMessageW(other, WM_PAINT, 0, 0);
            }
        }
        true
    }
    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.commands.borrow_mut())
    }
    fn take_task_commands(&mut self) -> Vec<crate::task::TaskCommand> {
        std::mem::take(&mut self.task_commands.borrow_mut())
    }
    fn task_completed(&mut self, result: crate::task::TaskResult) -> bool {
        let id = result.id;
        let value = result.into_value::<u32>().expect("native task failed");
        self.completed
            .borrow_mut()
            .push((id, value, std::thread::current().id()));
        if self.exit_on_completion {
            self.commands.borrow_mut().push(WindowCommand::Exit);
        }
        self.count.set(u64::from(value));
        !self.exit_on_completion // La sortie doit fonctionner sans repaint.
    }
}
pub(super) fn app() -> TestApp {
    TestApp {
        count: Rc::new(Cell::new(0)),
        drops: Rc::new(Cell::new(0)),
        commands: Rc::new(RefCell::new(Vec::new())),
        nested_target: Rc::new(Cell::new(null_mut())),
        fail_load: false,
        exit_on_update: false,
        task_commands: Rc::new(RefCell::new(Vec::new())),
        completed: Rc::new(RefCell::new(Vec::new())),
        exit_on_completion: false,
    }
}
pub(super) fn descriptor(id: u64, application: TestApp) -> WindowDescriptor {
    WindowDescriptor::new(
        WindowId(id),
        WindowOptions {
            title: "Karbon UI - test caché",
            width: 300,
            height: 220,
            min_width: 200,
            min_height: 160,
            activate: false,
            transparent: true,
            renderer: RenderMode::Software,
            ..WindowOptions::DEFAULT
        },
        application,
    )
}
pub(super) fn session() -> (ComApartment, Session) {
    let apartment = ComApartment::initialize().unwrap();
    let session = Session::new().unwrap();
    (apartment, session)
}

// Le secours évite de suspendre la suite si le réveil de GetMessage régresse.
pub(super) fn sent_message_while_waiting(
    window: Hwnd,
    message: u32,
    wparam: usize,
    run: impl FnOnce(),
) {
    // SAFETY : simple lecture de l'identifiant du thread courant.
    let thread = unsafe { GetCurrentThreadId() };
    let handle = window as usize;
    let (done, completed) = std::sync::mpsc::channel();
    let sender = std::thread::spawn(move || {
        // SAFETY : fenêtre de test de ce thread ; aucun pointeur dans wparam/lparam.
        unsafe {
            SendMessageW(handle as Hwnd, message, wparam, 0);
        }
        if completed
            .recv_timeout(std::time::Duration::from_secs(2))
            .is_err()
        {
            // SAFETY : message sans pointeur vers la file du thread UI.
            unsafe {
                PostThreadMessageW(thread, WM_QUIT, 0, 0);
            } // Sortie de secours.
            true
        } else {
            false
        }
    });
    run();
    done.send(()).unwrap();
    assert!(
        !sender.join().unwrap(),
        "GetMessage n'a pas été réveillé par le moteur"
    );
}
