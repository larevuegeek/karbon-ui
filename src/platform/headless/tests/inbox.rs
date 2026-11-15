//! Boîte aux lettres de bout en bout : messages et fermetures déposés par un autre
//! thread, appliqués par le runtime et visibles dans la vue.
use crate::{
    application::{Application, ViewContext},
    platform::headless::HeadlessApp,
    task::Inbox,
    ui::{element::Element, prelude::text},
};

#[derive(Clone)]
enum Msg {
    Add(u32),
}

struct Counter {
    inbox: Inbox<Counter>,
    total: u32,
}
impl Application for Counter {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        text("total", format!("Total {}", self.total))
    }
    fn update(&mut self, message: Msg) -> bool {
        let Msg::Add(n) = message;
        self.total += n;
        true
    }
    fn inbox(&self) -> Option<Inbox<Self>> {
        Some(self.inbox.clone())
    }
}

#[test]
fn posts_from_another_thread_reach_the_view() {
    let counter = Counter {
        inbox: Inbox::new(),
        total: 0,
    };
    let sender = counter.inbox.sender();
    let mut app = HeadlessApp::new(counter).unwrap();
    assert!(app.contains_text("Total 0"));
    std::thread::spawn(move || {
        sender.send(Msg::Add(2));
        sender.post(|counter| {
            counter.total *= 10;
            true
        });
    })
    .join()
    .unwrap();
    app.run_tasks().unwrap();
    assert_eq!(app.app().total, 20);
    assert!(app.contains_text("Total 20"));
    // Une seconde salve, après l'abonnement : même chemin.
    app.app().inbox.sender().send(Msg::Add(1));
    app.run_tasks().unwrap();
    assert!(app.contains_text("Total 21"));
}
